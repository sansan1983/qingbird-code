use std::sync::Arc;

use async_trait::async_trait;
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
use tokio::sync::Mutex;

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
                }),
                openai: None,
            },
            routing: RoutingConfig {
                strong: "anthropic".into(),
                medium: "anthropic".into(),
                light: "anthropic".into(),
            },
            cache: CacheConfig { l1_enabled: true },
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

// ========== 构造与状态 ==========

#[test]
fn new_creates_empty_active_agent() {
    let (o, _events) = make_orchestrator();
    assert!(o.active_agent.is_none());
}

#[tokio::test]
async fn ensure_agent_creates_default_subagent_lazily() {
    let (mut o, _events) = make_orchestrator();
    assert!(o.active_agent.is_none());
    {
        let _ = o.ensure_agent();
        assert!(o.active_agent.is_some());
    }
    let agent = o.active_agent.as_ref().unwrap();
    assert_eq!(agent.name, "default");
    assert!(matches!(agent.role, Role::Generalist));
    assert_eq!(agent.capabilities.len(), 3);
}

// ========== decompose 规则路径（L0/L1 + 短描述）==========

#[tokio::test]
async fn decompose_l0_short_returns_single_llm_reasoning_step() {
    let (o, _events) = make_orchestrator();
    let task = TaskSpec::new("read the README".into(), RiskLevel::L0);
    let plan = o.decompose(&task).await.unwrap();

    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.estimated_steps, 1);
    assert_eq!(plan.steps[0].tool, "llm_reasoning");
    assert_eq!(plan.steps[0].action, "read the README");
    assert_eq!(plan.steps[0].order, 0);
    assert!(plan.steps[0].depends_on.is_none());
    assert_eq!(plan.risk_level, RiskLevel::L0);
}

#[tokio::test]
async fn decompose_l1_short_returns_single_step() {
    let (o, _events) = make_orchestrator();
    let task = TaskSpec::new("write a log entry".into(), RiskLevel::L1);
    let plan = o.decompose(&task).await.unwrap();

    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].tool, "llm_reasoning");
    assert_eq!(plan.risk_level, RiskLevel::L1);
}

// ========== decompose LLM 路径（complex — 失败）==========

#[tokio::test]
async fn decompose_long_description_routes_to_llm_and_fails() {
    let (o, _events) = make_orchestrator();
    // 描述长度 > 100 字符 → 走 LLM 分解（dummy key 不可用 → 必然 Err）
    let long_desc: String = "a".repeat(200);
    let task = TaskSpec::new(long_desc, RiskLevel::L0);
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), o.decompose(&task))
        .await
        .expect("decompose should not hang");
    assert!(result.is_err());
}

#[tokio::test]
async fn decompose_l2_risk_routes_to_llm_and_fails() {
    let (o, _events) = make_orchestrator();
    // L2+ 风险无论长度都走 LLM 路径
    let task = TaskSpec::new("deploy service".into(), RiskLevel::L2);
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), o.decompose(&task))
        .await
        .expect("decompose should not hang");
    assert!(result.is_err());
}

#[tokio::test]
async fn decompose_l3_risk_routes_to_llm_and_fails() {
    let (o, _events) = make_orchestrator();
    let task = TaskSpec::new("delete production db".into(), RiskLevel::L3);
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), o.decompose(&task))
        .await
        .expect("decompose should not hang");
    assert!(result.is_err());
}

// ========== execute 事件发布 ==========

#[tokio::test]
async fn execute_publishes_task_started_event_first() {
    let (mut o, events) = make_orchestrator();
    let mut rx = events.subscribe();

    let task = TaskSpec::new("readme".into(), RiskLevel::L0);
    let task_id = task.id;

    // 预期在 execute_step 失败（llm_reasoning 调 LLM 不可用）— 加 5s 超时防挂死
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), o.execute(task)).await;

    // 第一个事件必须是 TaskStarted
    let event = rx.recv().await.unwrap();
    match event {
        Event::TaskStarted {
            task_id: id,
            description,
        } => {
            assert_eq!(id, task_id);
            assert_eq!(description, "readme");
        }
        other => panic!("expected TaskStarted, got {:?}", other),
    }
}

#[tokio::test]
async fn execute_does_not_publish_completed_on_failure() {
    // 失败路径不发 TaskCompleted（plan 字面如此；TaskFailed 也未发，spec 10.1 标记为后续工作）
    let (mut o, events) = make_orchestrator();
    let mut rx = events.subscribe();

    let task = TaskSpec::new("readme".into(), RiskLevel::L0);
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), o.execute(task)).await;
    let _ = result; // 不论是否超时，只检查事件

    // 收到 TaskStarted 后再尝试 recv 应该 pending（没有 TaskCompleted 也没有 TaskFailed）
    let event = rx.recv().await.unwrap();
    assert!(matches!(event, Event::TaskStarted { .. }));

    // 用 timeout 验证：下一个 recv 不会立即拿到 TaskCompleted
    let next = tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv()).await;
    assert!(
        next.is_err() || matches!(next, Ok(Err(_))),
        "expected timeout or channel closed, got: {:?}",
        next
    );
}
