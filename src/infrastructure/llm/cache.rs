//! M8 L2 结构化缓存 — 类型与 backend（L1 由 router 内联实现，不在此处）
//!
//! 设计见 spec §8.2-8.4。Key 由 (intent_type, task_signature, context_profile, model)
//! 四元组派生 64-bit hash。ContextProfile 存分桶而非原始值，保证泛化。
//! Value 三种形态：Decision / Execution / Feedback，对应能力层三段管线产物。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use lru::LruCache;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::common::types::{IntentType, RiskLevel};

/// L2 缓存 Key（设计 §8.3）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    pub intent_type: IntentType,
    pub task_signature: String,
    pub context_profile: ContextProfile,
    pub model: String,
}

/// 上下文特征（不存具体内容，存 bucket）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextProfile {
    /// 会话长度分桶：0=0-5, 1=6-20, 2=21-100, 3=100+
    pub conversation_depth_bucket: u8,
    /// 涉及文件数分桶：0=0, 1=1-3, 2=4-10, 3=10+
    pub file_count_bucket: u8,
    pub risk_level: RiskLevel,
    pub profile_name: String,
}

/// 缓存值（设计 §8.4）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheValue {
    Decision {
        plan_summary: String,
        risk: RiskLevel,
        model_choice: String,
    },
    Execution {
        result_summary: String,
        success: bool,
        duration_ms: u64,
    },
    Feedback {
        verdict_summary: String,
        confidence: f32,
    },
}

/// Key 派生一个 64-bit 哈希用于存储
pub fn key_hash(key: &CacheKey) -> u64 {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    h.finish()
}

/// 内存 LRU backend
pub struct MemoryLruBackend {
    cache: Mutex<LruCache<u64, (CacheValue, std::time::SystemTime)>>,
}

impl MemoryLruBackend {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).expect("capacity > 0"),
            )),
        }
    }

    pub fn get(&self, hash: u64) -> Option<CacheValue> {
        let mut c = self.cache.lock().ok()?;
        c.pop(&hash).map(|(v, _)| v)
    }

    pub fn put(&self, hash: u64, value: CacheValue) {
        if let Ok(mut c) = self.cache.lock() {
            c.put(hash, (value, std::time::SystemTime::now()));
        }
    }

    pub fn len(&self) -> usize {
        self.cache.lock().map(|c| c.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// SQLite 磁盘 backend（设计 §8.2 7 天 TTL）
pub struct SqliteCacheBackend {
    conn: Mutex<Connection>,
    ttl_secs: u64,
}

impl SqliteCacheBackend {
    pub fn open(path: &Path, ttl_days: u32) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS llm_cache (
                hash INTEGER PRIMARY KEY,
                value BLOB NOT NULL,
                created_at INTEGER NOT NULL
            )",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
            ttl_secs: u64::from(ttl_days) * 86400,
        })
    }

    pub fn get(&self, hash: u64) -> Option<CacheValue> {
        let conn = self.conn.lock().ok()?;
        let (blob, created_at): (Vec<u8>, i64) = conn
            .query_row(
                "SELECT value, created_at FROM llm_cache WHERE hash = ?1",
                params![hash as i64],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok()?;
        let now = unix_now();
        if now - created_at > self.ttl_secs as i64 {
            // 过期
            let _ = conn.execute(
                "DELETE FROM llm_cache WHERE hash = ?1",
                params![hash as i64],
            );
            return None;
        }
        serde_json::from_slice(&blob).ok()
    }

    pub fn put(&self, hash: u64, value: &CacheValue) -> Result<(), rusqlite::Error> {
        let blob = serde_json::to_vec(value)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let now = unix_now();
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return Ok(()), // ponytail: 锁失败 = 放弃缓存写入（best-effort）
        };
        conn.execute(
            "INSERT OR REPLACE INTO llm_cache (hash, value, created_at) VALUES (?1, ?2, ?3)",
            params![hash as i64, blob, now],
        )?;
        Ok(())
    }

    pub fn cleanup_expired(&self) -> Result<usize, rusqlite::Error> {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return Ok(0), // ponytail: 锁失败 = 放弃清理
        };
        let now = unix_now();
        let cutoff = now - self.ttl_secs as i64;
        let n = conn.execute(
            "DELETE FROM llm_cache WHERE created_at < ?1",
            params![cutoff],
        )?;
        Ok(n)
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// v1.2 D1: 把 capability 三处内联 CacheKey 构造抽到一处。
///
/// 设计选择（v1.1 plan §B1 + 跨阶段 D4 回顾）：
/// - `task_signature` **只含** step.action + step.tool，不含 params
///   （params 含时间戳/路径/任何字节 → cache miss；user 体验是「为什么我第二次没命中」）
/// - `intent_type` 由调用方按角色（Decisioner/Executor/Feedbacker）传入
/// - `model` 留空：Router 在 chat_cached 入口再注入
/// - `retry_count: Option<u8>`——v1.2 选 1 决策：
///   - Decisioner 传 `Some(retry_count)`：保持 v1.1 行为，break rework loop
///     （v1.1 注释：「retry_count 必含：break rework loop」）
///   - Executor / Feedbacker 传 `None`：retry 不再进 signature
///     - Executor：v1.1 注释说 step.action 在 rework 时被 subagent 追加建议，key 自动变
///     - Feedbacker：v1.1 把 retry_count 拼进 task_signature → 每次 retry 都 miss，
///       浪费 L2；v1.2 改成 retry 后 cache key 稳定（行为变化，commit body 明文记录）
#[must_use]
pub fn cache_key_for_step(
    step: &crate::common::types::TaskStep,
    intent_type: crate::common::types::IntentType,
    risk_level: crate::common::types::RiskLevel,
    profile_name: &str,
    retry_count: Option<u8>,
) -> CacheKey {
    let base = format!(
        "{}:{}:{}",
        intent_label(intent_type),
        step.tool,
        step.action
    );
    let task_signature = match retry_count {
        Some(r) => format!("{base}:retry={r}"),
        None => base,
    };
    CacheKey {
        intent_type,
        task_signature,
        context_profile: ContextProfile {
            conversation_depth_bucket: 0,
            file_count_bucket: 0,
            risk_level,
            profile_name: profile_name.to_string(),
        },
        model: String::new(),
    }
}

fn intent_label(it: crate::common::types::IntentType) -> &'static str {
    use crate::common::types::IntentType;
    match it {
        IntentType::CodeReview => "code_review",
        IntentType::BugFix => "bug_fix",
        IntentType::DataQuery => "data_query",
        IntentType::FileRead => "file_read",
        IntentType::FileWrite => "file_write",
        IntentType::CommandExecute => "command_execute",
        IntentType::WebFetch => "web_fetch",
        IntentType::Chat => "chat",
        IntentType::Unknown => "unknown",
    }
}

/// L2 缓存管理器（双层：内存 → 磁盘）
pub struct L2CacheManager {
    memory: MemoryLruBackend,
    disk: SqliteCacheBackend,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl L2CacheManager {
    pub fn new(
        memory_capacity: usize,
        disk_path: &Path,
        ttl_days: u32,
    ) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            memory: MemoryLruBackend::new(memory_capacity),
            disk: SqliteCacheBackend::open(disk_path, ttl_days)?,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        })
    }

    pub fn lookup(&self, key: &CacheKey) -> Option<CacheValue> {
        let h = key_hash(key);
        if let Some(v) = self.memory.get(h) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Some(v);
        }
        if let Some(v) = self.disk.get(h) {
            // 回填内存
            self.memory.put(h, v.clone());
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Some(v);
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    pub fn store(&self, key: &CacheKey, value: CacheValue) {
        let h = key_hash(key);
        self.memory.put(h, value.clone());
        let _ = self.disk.put(h, &value);
    }

    pub fn stats(&self) -> CacheStats {
        let h = self.hits.load(Ordering::Relaxed);
        let m = self.misses.load(Ordering::Relaxed);
        let total = h + m;
        let hit_rate = if total == 0 {
            0.0
        } else {
            h as f64 / total as f64
        };
        CacheStats {
            hits: h,
            misses: m,
            hit_rate,
        }
    }
}

/// 缓存统计（设计 §8.5 命中率上报）
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key() -> CacheKey {
        CacheKey {
            intent_type: IntentType::CodeReview,
            task_signature: "sig".into(),
            context_profile: ContextProfile {
                conversation_depth_bucket: 0,
                file_count_bucket: 0,
                risk_level: RiskLevel::L0,
                profile_name: "dev".into(),
            },
            model: "m".into(),
        }
    }

    #[test]
    fn cache_key_hash_is_stable() {
        let k = CacheKey {
            intent_type: IntentType::CodeReview,
            task_signature: "read_file_find_pattern".into(),
            context_profile: ContextProfile {
                conversation_depth_bucket: 1,
                file_count_bucket: 1,
                risk_level: RiskLevel::L0,
                profile_name: "developer".into(),
            },
            model: "claude-sonnet-4-6".into(),
        };
        assert_eq!(key_hash(&k), key_hash(&k));
    }

    #[test]
    fn cache_key_differs_on_model() {
        let mut k1 = make_key();
        k1.model = "claude-sonnet-4-6".into();
        let mut k2 = make_key();
        k2.model = "claude-opus-4-8".into();
        assert_ne!(key_hash(&k1), key_hash(&k2));
    }

    // v1.2 D1: cache_key_for_step helper signature 是 (step, intent, risk, profile, retry_count)
    // 选 1 决策：retry_count: Option<u8>——Decisioner 传 Some 保持 v1.1 break-rework-loop 行为；
    // Feedbacker 传 None 实现 retry 后 cache key 稳定。Executor 传 None（v1.1 注释说它的
    // retry 通过 step.action 变化处理）。
    #[test]
    fn cache_key_for_step_uses_action_and_tool_only() {
        // helper 不应把 step.params 序列化进 signature
        // （params 任一字节变 → cache miss；user 改个时间戳就破命中）
        // signature 应只含 action + tool
        let step = crate::common::types::TaskStep {
            action: "review code".into(),
            tool: "read_file".into(),
            params: serde_json::json!({"path": "/tmp/foo.rs"}),
            expected_output: None,
        };
        let key = cache_key_for_step(
            &step,
            crate::common::types::IntentType::CodeReview,
            crate::common::types::RiskLevel::L0,
            "default",
            None,
        );
        assert_eq!(key.task_signature, "code_review:read_file:review code");
        assert!(
            key.model.is_empty(),
            "model 字段由 Router 注入，不在 helper 写死"
        );
    }

    // v1.2 D2: action 变化必须产生不同 hash（确保 D1 helper 真的把 action 进了 signature）
    #[test]
    fn cache_key_hash_differs_on_action_change() {
        let k1 = cache_key_for_step(
            &crate::common::types::TaskStep {
                action: "read foo.rs".into(),
                tool: "read_file".into(),
                params: serde_json::json!({}),
                expected_output: None,
            },
            crate::common::types::IntentType::FileRead,
            crate::common::types::RiskLevel::L0,
            "developer",
            None,
        );
        let k2 = cache_key_for_step(
            &crate::common::types::TaskStep {
                action: "read bar.rs".into(),
                tool: "read_file".into(),
                params: serde_json::json!({}),
                expected_output: None,
            },
            crate::common::types::IntentType::FileRead,
            crate::common::types::RiskLevel::L0,
            "developer",
            None,
        );
        assert_ne!(key_hash(&k1), key_hash(&k2));
    }

    // v1.2 D2: params 变化（即使 JSON 完全不同）应产生相同 hash
    // ——这是 D1 的核心收益，避免 user 改个时间戳就破命中
    #[test]
    fn cache_key_hash_ignores_params_byte_changes() {
        let k1 = cache_key_for_step(
            &crate::common::types::TaskStep {
                action: "review".into(),
                tool: "read_file".into(),
                params: serde_json::json!({"path": "/tmp/a.rs", "timestamp": 1000}),
                expected_output: None,
            },
            crate::common::types::IntentType::CodeReview,
            crate::common::types::RiskLevel::L0,
            "developer",
            None,
        );
        let k2 = cache_key_for_step(
            &crate::common::types::TaskStep {
                action: "review".into(),
                tool: "read_file".into(),
                params: serde_json::json!({"path": "/tmp/a.rs", "timestamp": 9999}),
                expected_output: None,
            },
            crate::common::types::IntentType::CodeReview,
            crate::common::types::RiskLevel::L0,
            "developer",
            None,
        );
        assert_eq!(
            key_hash(&k1),
            key_hash(&k2),
            "params 变化不应改变 cache key"
        );
    }
}

#[cfg(test)]
mod memory_lru_tests {
    use super::*;

    #[test]
    fn lru_put_then_get_returns_value() {
        let b = MemoryLruBackend::new(10);
        b.put(
            1,
            CacheValue::Execution {
                result_summary: "x".into(),
                success: true,
                duration_ms: 100,
            },
        );
        match b.get(1) {
            Some(CacheValue::Execution { result_summary, .. }) => assert_eq!(result_summary, "x"),
            _ => panic!("expected Execution"),
        }
    }

    #[test]
    fn lru_evicts_oldest_when_full() {
        let b = MemoryLruBackend::new(2);
        b.put(
            1,
            CacheValue::Execution {
                result_summary: "a".into(),
                success: true,
                duration_ms: 1,
            },
        );
        b.put(
            2,
            CacheValue::Execution {
                result_summary: "b".into(),
                success: true,
                duration_ms: 1,
            },
        );
        b.put(
            3,
            CacheValue::Execution {
                result_summary: "c".into(),
                success: true,
                duration_ms: 1,
            },
        );
        assert!(b.get(1).is_none()); // 被淘汰
        assert!(b.get(2).is_some());
        assert!(b.get(3).is_some());
    }
}

#[cfg(test)]
mod sqlite_cache_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn sqlite_put_then_get_returns_value() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("cache.db");
        let b = SqliteCacheBackend::open(&path, 7).unwrap();
        let v = CacheValue::Execution {
            result_summary: "x".into(),
            success: true,
            duration_ms: 100,
        };
        b.put(42, &v).unwrap();
        match b.get(42) {
            Some(CacheValue::Execution { result_summary, .. }) => {
                assert_eq!(result_summary, "x");
            }
            _ => panic!("expected Execution"),
        }
    }

    #[test]
    fn sqlite_expired_entries_returned_as_none() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("cache.db");
        // ttl_days=0 立即过期
        let b = SqliteCacheBackend::open(&path, 0).unwrap();
        let v = CacheValue::Execution {
            result_summary: "x".into(),
            success: true,
            duration_ms: 1,
        };
        b.put(99, &v).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert!(b.get(99).is_none());
    }

    #[test]
    fn sqlite_cleanup_expired_removes_old_entries() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("cache.db");
        let b = SqliteCacheBackend::open(&path, 0).unwrap();
        let v = CacheValue::Execution {
            result_summary: "x".into(),
            success: true,
            duration_ms: 1,
        };
        b.put(1, &v).unwrap();
        b.put(2, &v).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let removed = b.cleanup_expired().unwrap();
        assert_eq!(removed, 2);
    }
}

#[cfg(test)]
mod manager_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn manager_lookup_miss_then_hit_after_store() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("cache.db");
        let m = L2CacheManager::new(100, &path, 7).unwrap();
        let key = CacheKey {
            intent_type: IntentType::CodeReview,
            task_signature: "x".into(),
            context_profile: ContextProfile {
                conversation_depth_bucket: 0,
                file_count_bucket: 0,
                risk_level: RiskLevel::L0,
                profile_name: "dev".into(),
            },
            model: "m".into(),
        };
        assert!(m.lookup(&key).is_none()); // miss
        m.store(
            &key,
            CacheValue::Execution {
                result_summary: "v".into(),
                success: true,
                duration_ms: 1,
            },
        );
        assert!(m.lookup(&key).is_some()); // hit
        let s = m.stats();
        assert_eq!(s.hits, 1);
        assert_eq!(s.misses, 1);
        assert!((s.hit_rate - 0.5).abs() < 0.01);
    }
}
