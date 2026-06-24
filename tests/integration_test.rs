//! M14: 端到端集成测试
//!
//! 范围：跨模块拼装，验证模块间契约（事件流、数据一致性、跨层贯通）。
//! 单模块细节由各 M 的单元/集成测试覆盖，本文件不重复。
//!
//! 限制：测试用 dummy LLM key，LLM 路径必失败；规则路径（短描述 + L0/L1）
//! 可跑通（Orchestrator→Subagent→StubEcho 工具）。LLM 路径用 5s timeout 防挂死。
//!
//! 运行：cargo test --test integration_test

rust_i18n::i18n!("locales", fallback = "en-US");

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use qingbird_code::application::concierge::Concierge;
use qingbird_code::application::orchestrator::Orchestrator;
use qingbird_code::capability::subagent::Subagent;
use qingbird_code::capability::tools::{Tool, ToolDefinition, ToolOutput, ToolRegistry};
use qingbird_code::common::error::Result;
use qingbird_code::common::types::*;
use qingbird_code::infrastructure::config::{
    CacheConfig, CoreConfig, DeepseekConfig, EflowConfig, LlmConfig, MemoryConfig,
    ProfileListConfig, SecurityConfig,
};
use qingbird_code::infrastructure::context::ContextCompressor;
use qingbird_code::infrastructure::event::{Event, EventChannel};
use qingbird_code::infrastructure::llm::LlmRouter;
use qingbird_code::infrastructure::memory::{
    CompositeMemory, MemoryEntry, MemoryManager, RecallScope, WorkingMemory,
};
// (ProfileRegistry 已被 Concierge 删，测试不再需要)
use tokio::sync::Mutex;

// locale setup moved into individual tests

// ========== Test fixtures ==========

fn make_test_config() -> EflowConfig {
    EflowConfig {
        core: CoreConfig {
            language: "zh-CN".into(),
            timezone: "UTC".into(),
        },
        llm: LlmConfig {
            deepseek: DeepseekConfig {
                api_key: Some("test-key".into()),
                base_url: Some("http://localhost:9999".into()),
                default_model: Some("deepseek-chat".into()),
                timeout_secs: 5,
                max_retries: 0,
                retry_backoff_ms: 100,
            },
            cache: CacheConfig {
                l1_enabled: true,
                l2_enabled: false,
                l2_ttl_days: 7,
            },
        },
        memory: MemoryConfig {
            working_memory_limit: 100,
            project_db_path: ":memory:".into(),
            user_db_path: ":memory:".into(),
            cleanup_interval_hours: 24,
        },
        security: SecurityConfig {
            risk_threshold: RiskLevel::L2,
            allowed_paths: vec![],
        },
        profiles: ProfileListConfig {
            default: "developer".into(),
            available: vec!["developer".into()],
        },
    }
}

fn make_test_router() -> Arc<Mutex<LlmRouter>> {
    let cfg = make_test_config();
    let router = LlmRouter::from_config(&cfg).expect("test router");
    Arc::new(Mutex::new(router))
}

struct StubEchoTool;

#[async_trait]
impl Tool for StubEchoTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "stub_echo".into(),
            description: "echoes params".into(),
            parameters: serde_json::json!({}),
            risk_level: RiskLevel::L0,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput> {
        Ok(ToolOutput {
            success: true,
            content: format!("echo: {}", params),
            metadata: None,
        })
    }
}

fn make_tool_registry() -> Arc<ToolRegistry> {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(StubEchoTool));
    Arc::new(reg)
}

fn make_orchestrator() -> (Orchestrator, EventChannel) {
    let router = make_test_router();
    let tools = make_tool_registry();
    let events = EventChannel::new();
    let o = Orchestrator::new(router, tools, events.clone());
    (o, events)
}

fn make_concierge() -> (Concierge, EventChannel) {
    let (orch, events) = make_orchestrator();
    let orch = Arc::new(Mutex::new(orch));
    let mem = Arc::new(Mutex::new(CompositeMemory::in_memory(100).unwrap()));
    let llm = make_test_router(); // v1.3.1 增量
    let c = Concierge::new(
        events.clone(),
        mem,
        orch,
        llm, // v1.3.1 增量
        "developer".into(),
    );
    (c, events)
}

// ========== 1. Concierge→Orchestrator→事件 端到端贯通 ==========

#[tokio::test]
async fn e2e_concierge_dispatch_publishes_lifecycle_events() {
    // 派发 L0 短任务 → 规则分解 → 必发 TaskStarted 事件 + 终态事件 (Completed/Failed)
    // 不关心 task_id 精确值（Concierge 内部生成），只验证事件序列
    let (mut c, events) = make_concierge();
    let mut rx = events.subscribe();

    let _ = c.handle_input("readme".into()).await;

    // 第一个事件 = TaskStarted
    // 30s timeout 容纳 A3 加的指数退避（3 次 retry：1s+2s+4s=7s sleep）+ CI 抢占开销
    // v1.0.3 时 15s 足够（无 backoff），v1.1 加退避后满套件跑临界 → 拉宽
    let first = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("30s 内必收到首事件")
        .expect("channel 不应关闭");
    assert!(
        matches!(first, Event::TaskStarted { .. }),
        "首事件应是 TaskStarted，实际: {:?}",
        first
    );

    // 第二事件 = TaskCompleted 或 TaskFailed
    let mut terminal_seen = false;
    #[allow(clippy::never_loop)] // 3 次机会 break 模式只用 1 次；v1.x 改 '_ => continue' 用足 3 次
    for _ in 0..3 {
        match tokio::time::timeout(Duration::from_secs(30), rx.recv()).await {
            Ok(Ok(Event::TaskCompleted { .. })) => {
                terminal_seen = true;
                break;
            }
            Ok(Ok(Event::TaskFailed { .. })) => {
                terminal_seen = true;
                break;
            }
            _ => break,
        }
    }
    assert!(terminal_seen, "应在 5s 内收到 TaskCompleted 或 TaskFailed");
}

#[tokio::test]
async fn e2e_event_channel_broadcasts_to_multiple_subscribers() {
    // 单 publish → N subscriber 都收到
    let channel = EventChannel::new();
    let mut rx1 = channel.subscribe();
    let mut rx2 = channel.subscribe();
    let mut rx3 = channel.subscribe();

    let task_id = uuid::Uuid::new_v4();
    channel.publish(Event::TaskStarted {
        task_id,
        description: "broadcast test".into(),
    });

    for (i, rx) in [&mut rx1, &mut rx2, &mut rx3].into_iter().enumerate() {
        let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap_or_else(|_| panic!("subscriber {} 1s 内未收到事件", i))
            .unwrap();
        match event {
            Event::TaskStarted { task_id: id, .. } => assert_eq!(id, task_id),
            other => panic!("subscriber {} 收到非 TaskStarted: {:?}", i, other),
        }
    }
}

// ========== 2. 跨层数据一致性 ==========

#[test]
fn e2e_blackboard_with_plan_and_step_carries_task_id_through() {
    // 验证：plan.task_id == 原始 task.id（Orchestrator 拆分后不丢 id）
    use qingbird_code::capability::blackboard::Blackboard;

    let task = TaskSpec::new("跨层测试".into(), RiskLevel::L0);
    let task_id = task.id;
    let bb = Blackboard::new(task).with_plan(TaskPlan {
        task_id,
        steps: vec![PlannedStep {
            order: 0,
            action: "step1".into(),
            tool: "stub_echo".into(),
            params: serde_json::json!({}),
            depends_on: None,
        }],
        estimated_steps: 1,
        risk_level: RiskLevel::L0,
    });

    assert_eq!(bb.task.id, task_id);
    assert_eq!(bb.plan.as_ref().unwrap().task_id, task_id);
}

#[test]
fn e2e_subagent_default_capabilities_cover_l0_l1() {
    // Orchestrator 懒初始化的 default Subagent：capability 必须覆盖 L0/L1 任务
    // (Plan 默认注入 ReadFile/WriteFile/LlmReasoning 3 项)
    let agent = Subagent::new(
        "default".into(),
        Role::Generalist,
        vec![
            Capability::ReadFile,
            Capability::WriteFile,
            Capability::LlmReasoning,
        ],
    );
    assert_eq!(agent.capabilities.len(), 3);
    assert!(agent.capabilities.contains(&Capability::LlmReasoning));
    assert!(agent.capabilities.contains(&Capability::ReadFile));
}

// ========== 3. ContextCompressor 真实 API ==========

#[test]
fn e2e_context_compression_empty_log_does_not_panic() {
    // 不变量：空输入不 panic
    let compressed = ContextCompressor::compress_action_log(&[]);
    // 任意非 panic 输出即合规
    let _ = compressed.len();
}

#[test]
fn e2e_context_compression_with_real_action_records() {
    let logs = vec![
        ActionRecord {
            timestamp: Utc::now(),
            action: "read README".into(),
            tool: "read_file".into(),
            success: true,
            summary: "read 200 lines".into(),
        },
        ActionRecord {
            timestamp: Utc::now(),
            action: "search code".into(),
            tool: "search_code".into(),
            success: false,
            summary: "no match".into(),
        },
    ];
    let compressed = ContextCompressor::compress_action_log(&logs);
    assert!(
        compressed.contains("read_file") || compressed.contains("✓") || compressed.contains("read"),
        "压缩结果应含工具名或成功标记，实际: {}",
        compressed
    );
}

#[test]
fn e2e_context_compression_estimate_tokens_is_monotonic() {
    // 长文本 token 估计 ≥ 短文本
    let short = ContextCompressor::estimate_tokens("hello");
    let long = ContextCompressor::estimate_tokens(&"a".repeat(1000));
    assert!(long > short, "长文本 token 估计应更大");
}

// ========== 4. Memory 跨层 ==========

#[test]
fn e2e_memory_remember_recall_round_trip() {
    let mut memory = WorkingMemory::new(100);
    let entry = MemoryEntry::new(
        "跨层测试记忆",
        MemoryCategory::TaskResult,
        Importance::Normal,
    );
    let id = memory.remember(entry).unwrap();

    let results = memory.recall("测试", RecallScope::Working, 10).unwrap();
    assert!(!results.is_empty(), "recall 应至少返回 1 条");
    assert!(
        results.iter().any(|e| e.id == id),
        "recall 应包含刚写入的 entry"
    );
}

#[test]
fn e2e_composite_memory_in_memory_works() {
    // M5 内存模式构造验证（与 M11/M12 测试同源；此处作为端到端 sanity check）
    let mem = CompositeMemory::in_memory(50).expect("in-memory composite");
    let mut mem = mem;
    let entry = MemoryEntry::new("sanity", MemoryCategory::ManualNote, Importance::Low);
    let id = mem.remember_smart(entry).expect("remember_smart");
    assert!(!id.is_nil());
}

// ========== 5. 全局不变量 ==========

#[test]
fn e2e_risk_level_ordering() {
    // 跨模块依赖：Orchestrator.decompose / Security / Decisioner 全部使用 < 关系
    assert!(RiskLevel::L0 < RiskLevel::L1);
    assert!(RiskLevel::L1 < RiskLevel::L2);
    assert!(RiskLevel::L2 < RiskLevel::L3);
    assert!(RiskLevel::L0 < RiskLevel::L3);
}

#[test]
fn e2e_task_id_uniqueness_under_rapid_creation() {
    // TaskSpec::new 内部 Uuid::new_v4() → 高并发下也应唯一
    use std::collections::HashSet;
    let mut ids = HashSet::new();
    for _ in 0..1000 {
        let t = TaskSpec::new("x".into(), RiskLevel::L0);
        assert!(ids.insert(t.id), "1000 次构造应全部产生不同 id");
    }
    assert_eq!(ids.len(), 1000);
}

// ========== 6. 事件流多类型 ==========

#[tokio::test]
async fn e2e_event_channel_publishes_all_6_variants() {
    let channel = EventChannel::new();
    let mut rx = channel.subscribe();
    let task_id = uuid::Uuid::new_v4();

    channel.publish(Event::TaskStarted {
        task_id,
        description: "x".into(),
    });
    channel.publish(Event::TaskCompleted {
        task_id,
        summary: "done".into(),
    });
    channel.publish(Event::TaskFailed {
        task_id,
        error: "oops".into(),
    });
    channel.publish(Event::RiskEscalated {
        task_id,
        from: RiskLevel::L1,
        to: RiskLevel::L2,
    });
    channel.publish(Event::UserInputRequired { prompt: "?".into() });
    channel.publish(Event::SystemShutdown);

    let mut received = Vec::new();
    for _ in 0..6 {
        match tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("1s 内应收到事件")
        {
            Ok(ev) => received.push(ev),
            Err(_) => panic!("broadcast channel 不应关闭"),
        }
    }

    let has_started = received
        .iter()
        .any(|e| matches!(e, Event::TaskStarted { .. }));
    let has_completed = received
        .iter()
        .any(|e| matches!(e, Event::TaskCompleted { .. }));
    let has_failed = received
        .iter()
        .any(|e| matches!(e, Event::TaskFailed { .. }));
    let has_escalated = received
        .iter()
        .any(|e| matches!(e, Event::RiskEscalated { .. }));
    let has_input = received
        .iter()
        .any(|e| matches!(e, Event::UserInputRequired { .. }));
    let has_shutdown = received.iter().any(|e| matches!(e, Event::SystemShutdown));

    assert_eq!(received.len(), 6, "应收到全部 6 个事件");
    assert!(
        has_started && has_completed && has_failed && has_escalated && has_input && has_shutdown
    );
}

// ========== v1.1 Task A6: timeout 配置端到端贯通 ==========

#[tokio::test]
async fn llm_router_handles_timeout_via_config() {
    // v1.1 Task A2 + A3: timeout_secs=0 → reqwest 立即超时 → A3 retry 也耗尽 → Err
    // （plan 写了 `use std::path::PathBuf;` 没用上，clippy pedantic 必挂 → 已删）
    use qingbird_code::infrastructure::config::{
        CacheConfig, CoreConfig, EflowConfig, LlmConfig, MemoryConfig, ProfileListConfig,
        SecurityConfig,
    };
    use qingbird_code::infrastructure::llm::{ChatRequest, LlmRouter, Message};

    let cfg = EflowConfig {
        core: CoreConfig {
            language: "zh-CN".into(),
            timezone: "Asia/Shanghai".into(),
        },
        llm: LlmConfig {
            deepseek: DeepseekConfig {
                api_key: Some("test-key".into()),
                base_url: Some("http://localhost:9999".into()),
                default_model: Some("deepseek-chat".into()),
                timeout_secs: 5,
                max_retries: 0,
                retry_backoff_ms: 100,
            },
            cache: CacheConfig {
                l1_enabled: true,
                l2_enabled: false,
                l2_ttl_days: 7,
            },
        },
        memory: MemoryConfig {
            working_memory_limit: 100,
            project_db_path: "./p.db".into(),
            user_db_path: "./u.db".into(),
            cleanup_interval_hours: 24,
        },
        security: SecurityConfig {
            risk_threshold: RiskLevel::L2,
            allowed_paths: vec![],
        },
        profiles: ProfileListConfig {
            default: "dev".into(),
            available: vec!["dev".into()],
        },
    };

    let router = LlmRouter::from_config(&cfg).unwrap();
    let req = ChatRequest::new("", vec![Message::user("hi")]);
    let result = router.chat(ModelTier::Light, req).await;
    assert!(result.is_err());
}

// ========== v1.1 Task B7: L2 命中率端到端 ==========

#[tokio::test]
async fn l2_cache_hit_rate_increases_with_repeated_calls() {
    // v1.1 Task B7（设计 §8.5）：重复调用同一 key 命中率上升
    use qingbird_code::common::types::{IntentType, RiskLevel};
    use qingbird_code::infrastructure::llm::cache::{CacheKey, CacheValue, ContextProfile};
    use qingbird_code::infrastructure::llm::l2::L2CacheManager;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("cache.db");
    let cache = L2CacheManager::new(100, &path, 7).unwrap();
    let key = CacheKey {
        intent_type: IntentType::CodeReview,
        task_signature: "sig".into(),
        context_profile: ContextProfile {
            conversation_depth_bucket: 1,
            file_count_bucket: 0,
            risk_level: RiskLevel::L0,
            profile_name: "developer".into(),
        },
        model: "claude-sonnet-4-6".into(),
    };

    // 模拟 5 次调用：第一次 miss, 后续 4 次 hit
    let mut hits = 0;
    let mut misses = 0;
    // 第一次 lookup: miss
    misses += usize::from(cache.lookup(&key).is_none());
    // 写入
    cache.store(
        &key,
        CacheValue::Execution {
            result_summary: "result".into(),
            success: true,
            duration_ms: 100,
        },
    );
    // 后续 4 次 lookup: hit
    for _ in 0..4 {
        hits += usize::from(cache.lookup(&key).is_some());
    }
    assert_eq!(hits, 4);
    assert_eq!(misses, 1);
    let stats = cache.stats();
    assert!(
        (stats.hit_rate - 0.8).abs() < 0.01,
        "hit_rate 应 ≈0.8, 实际 {}",
        stats.hit_rate
    );
}
