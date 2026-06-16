rust_i18n::i18n!("locales", fallback = "en-US");

use std::sync::Arc;

use async_trait::async_trait;
use eflow::application::concierge::Concierge;
use eflow::application::orchestrator::Orchestrator;
use eflow::capability::tools::{Tool, ToolDefinition, ToolOutput, ToolRegistry};
use eflow::common::error::Result;
use eflow::common::types::*;
use eflow::infrastructure::config::{
    CacheConfig, CoreConfig, EflowConfig, LlmConfig, MemoryConfig, ProfileListConfig,
    ProviderEntry, ProvidersConfig, RoutingConfig, SecurityConfig,
};
use eflow::infrastructure::event::{Event, EventChannel};
use eflow::infrastructure::llm::LlmRouter;
use eflow::infrastructure::memory::CompositeMemory;
use eflow::infrastructure::profile::ProfileRegistry;
use tokio::sync::{Mutex, RwLock};

// 默认中文 locale
// locale setup moved into individual tests

// ========== 测试辅助 ==========

fn make_test_config() -> EflowConfig {
    EflowConfig {
        core: CoreConfig {
            language: "zh-CN".into(),
            timezone: "UTC".into(),
        },
        llm: LlmConfig {
            providers: ProvidersConfig {
                anthropic: Some(ProviderEntry {
                    api_key: "test-key".into(),
                    default_model: "claude-test".into(),
                    timeout_secs: 30,
                    max_retries: 3,
                    retry_backoff_ms: 1000,
                }),
                openai: None,
            },
            routing: RoutingConfig {
                strong: "anthropic".into(),
                medium: "anthropic".into(),
                light: "anthropic".into(),
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
    // v1.1 跨阶段: 显式清掉 *BASE_URL env var，避免 dev shell 的 cc-connect 代理
    // 污染 test（详见 capability_test.rs 同名 helper）
    // SAFETY: 单线程测试构造时清 env var，无 race
    unsafe {
        std::env::remove_var("ANTHROPIC_BASE_URL");
        std::env::remove_var("OPENAI_BASE_URL");
    }
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

fn make_memory() -> Arc<Mutex<CompositeMemory>> {
    let mem = CompositeMemory::in_memory(100).expect("in-memory composite");
    Arc::new(Mutex::new(mem))
}

fn make_profiles() -> Arc<RwLock<ProfileRegistry>> {
    Arc::new(RwLock::new(ProfileRegistry::new()))
}

fn make_concierge() -> (Concierge, EventChannel) {
    let (orch, events) = make_orchestrator();
    let orch = Arc::new(Mutex::new(orch));
    let mem = make_memory();
    let profiles = make_profiles();
    let c = Concierge::new(events.clone(), mem, profiles, orch, "developer".into());
    (c, events)
}

// ========== 构造 ==========

#[test]
fn new_stores_fields() {
    let (c, _events) = make_concierge();
    // active_profile 通过 classify 行为间接验证：此处仅保证构造成功
    let intent = c.classify_intent("hello");
    assert!(matches!(intent, Intent::TaskDispatch { .. }));
}

// ========== classify_intent 6 路径 ==========

#[test]
fn classify_profile_switch_extracts_name_from_last_token() {
    let (c, _events) = make_concierge();
    match c.classify_intent("切换到 profile researcher") {
        Intent::ProfileSwitch { industry } => assert_eq!(industry, "researcher"),
        other => panic!("expected ProfileSwitch, got {:?}", other),
    }
}

#[test]
fn classify_task_cancel_returns_nil_id_in_v1() {
    let (c, _events) = make_concierge();
    match c.classify_intent("取消任务") {
        Intent::TaskCancel { task_id } => {
            assert_eq!(task_id, uuid::Uuid::nil(), "v1.0 不跟踪 task id，应为 nil");
        }
        other => panic!("expected TaskCancel, got {:?}", other),
    }
}

#[test]
fn classify_task_interrupt_returns_nil_id_in_v1() {
    let (c, _events) = make_concierge();
    match c.classify_intent("中断") {
        Intent::TaskInterrupt { task_id } => {
            assert_eq!(task_id, uuid::Uuid::nil());
        }
        other => panic!("expected TaskInterrupt, got {:?}", other),
    }
}

#[test]
fn classify_skill_query_keyword_english() {
    let (c, _events) = make_concierge();
    match c.classify_intent("list available skill") {
        Intent::SkillQuery { keyword } => assert_eq!(keyword, "list available skill"),
        other => panic!("expected SkillQuery, got {:?}", other),
    }
}

#[test]
fn classify_skill_query_keyword_chinese() {
    let (c, _events) = make_concierge();
    match c.classify_intent("查询所有技能") {
        Intent::SkillQuery { keyword } => assert_eq!(keyword, "查询所有技能"),
        other => panic!("expected SkillQuery, got {:?}", other),
    }
}

#[test]
fn classify_default_is_task_dispatch() {
    let (c, _events) = make_concierge();
    match c.classify_intent("读 README") {
        Intent::TaskDispatch { spec } => {
            assert_eq!(spec.description, "读 README");
            assert_eq!(spec.risk_level, RiskLevel::L0);
        }
        other => panic!("expected TaskDispatch, got {:?}", other),
    }
}

// ========== handle_input 行为 ==========

#[tokio::test]
async fn handle_input_default_routes_to_dispatch() {
    // v1.0 simplify: classify_intent 删了 Chat 路径，所有输入默认走 task dispatch
    let (c, _events) = make_concierge();
    let resp = c.handle_input("你好".into()).await;
    // 派发响应含 task id 或 "派发"/"dispatched" 字样
    assert!(
        resp.contains("派发") || resp.contains("dispatched"),
        "默认应派发为 task，响应: {}",
        resp
    );
}

#[tokio::test]
async fn handle_skill_query_returns_placeholder() {
    let (c, _events) = make_concierge();
    let resp = c.handle_input("list skill".into()).await;
    assert!(
        resp.contains("v1.0"),
        "skill query 应返回占位提示: {}",
        resp
    );
}

#[tokio::test]
async fn handle_task_dispatch_does_not_block_on_execution() {
    // L0 短任务应走规则分解 → llm_reasoning 会尝试调 LLM（必然失败）。
    // 但 handle_input 必须在 tokio::spawn 后立即返回，不等执行完成。
    let (c, _events) = make_concierge();

    let start = std::time::Instant::now();
    let resp = tokio::time::timeout(
        std::time::Duration::from_millis(200),
        c.handle_input("readme".into()),
    )
    .await
    .expect("handle_input should not block (spawned task runs async)");
    let elapsed = start.elapsed();

    assert!(elapsed < std::time::Duration::from_millis(200));
    assert!(
        resp.contains("readme") || resp.contains("派发") || resp.contains("dispatched"),
        "派发响应应含任务 id 或派发字样: {}",
        resp
    );
}

#[tokio::test]
async fn handle_task_dispatch_publishes_task_completed_or_failed_event() {
    // 派发后等子任务完成（成功或失败）→ 应收到 TaskCompleted 或 TaskFailed 事件
    let (c, events) = make_concierge();
    let mut rx = events.subscribe();

    let _ = c.handle_input("readme".into()).await;

    // 后台 task 会调 LLM（dummy key 失败）→ 期望 TaskFailed
    // 15s timeout 容纳 A3 加的指数退避（3 次 retry：1s+2s+4s=7s sleep）
    // 第一事件必然是 TaskStarted，跳过
    let _ = tokio::time::timeout(std::time::Duration::from_secs(15), rx.recv())
        .await
        .expect("应在 15s 内收到首个事件")
        .expect("channel 不应关闭");

    let event = tokio::time::timeout(std::time::Duration::from_secs(15), rx.recv())
        .await
        .expect("应在 15s 内收到 task 完成事件")
        .expect("channel 不应关闭");

    assert!(
        matches!(
            event,
            Event::TaskCompleted { .. } | Event::TaskFailed { .. }
        ),
        "派发后应收到 TaskCompleted 或 TaskFailed，实际: {:?}",
        event
    );
}
