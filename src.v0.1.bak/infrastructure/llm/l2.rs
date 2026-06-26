//! L2 持久化缓存（双层：内存 → 磁盘 SQLite）
//!
//! 设计见 spec §8.2 7 天 TTL。L2CacheManager 组合 L1 内存 + L2 SQLite，
//! lookup 先查内存再查磁盘，命中后回填内存。

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};
use serde::Serialize;

use super::cache_key::key_hash;
use super::l1::MemoryLruBackend;
use crate::infrastructure::llm::cache::{CacheKey, CacheValue};

/// SQLite 磁盘 backend
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
            Err(_) => return Ok(()), // best-effort
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
            Err(_) => return Ok(0),
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

/// 缓存统计
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod sqlite_tests {
    use super::*;
    use crate::infrastructure::llm::cache::CacheValue;
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
    use crate::common::types::RiskLevel;
    use crate::infrastructure::llm::cache::{CacheKey, ContextProfile};
    use tempfile::TempDir;

    #[test]
    fn manager_lookup_miss_then_hit_after_store() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("cache.db");
        let m = L2CacheManager::new(100, &path, 7).unwrap();
        let key = CacheKey {
            intent_type: crate::common::types::IntentType::CodeReview,
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
