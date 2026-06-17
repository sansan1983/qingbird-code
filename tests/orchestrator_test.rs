use std::sync::Arc;

use async_trait::async_trait;
use eflow::application::orchestrator::Orchestrator;
use eflow::capability::pool::SubagentPool;
use eflow::capability::tools::{Tool, ToolDefinition, ToolOutput, ToolRegistry};
use eflow::common::error::Result;
use eflow::common::types::*;
use eflow::infrastructure::config::{
    CacheConfig, CoreConfig, EflowConfig, LlmConfig, MemoryConfig, ProfileListConfig,
    RoutingConfig, SecurityConfig,
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
    // v1.3: 把 provider 写到临时 dir
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("anthropic.yaml"),
        "id: anthropic\ndisplay_name: Anthropic\nprotocol: anthropic_compatible\nbase_url: https://api.anthropic.com\napi_key: test-key\ndefault_model: claude-test\n",
    )
    .unwrap();
    let router = LlmRouter::from_config(&cfg, dir.path()).expect("test router");
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
    // 15s timeout 容纳 A3 加的指数退避（3 次 retry：1s+2s+4s=7s sleep）
    let long_desc: String = "a".repeat(200);
    let task = TaskSpec::new(long_desc, RiskLevel::L0);
    let result = tokio::time::timeout(std::time::Duration::from_secs(15), o.decompose(&task))
        .await
        .expect("decompose should not hang");
    assert!(result.is_err());
}

#[tokio::test]
async fn decompose_l2_risk_routes_to_llm_and_fails() {
    let (o, _events) = make_orchestrator();
    // L2+ 风险无论长度都走 LLM 路径
    let task = TaskSpec::new("deploy service".into(), RiskLevel::L2);
    let result = tokio::time::timeout(std::time::Duration::from_secs(15), o.decompose(&task))
        .await
        .expect("decompose should not hang");
    assert!(result.is_err());
}

#[tokio::test]
async fn decompose_l3_risk_routes_to_llm_and_fails() {
    let (o, _events) = make_orchestrator();
    let task = TaskSpec::new("delete production db".into(), RiskLevel::L3);
    let result = tokio::time::timeout(std::time::Duration::from_secs(15), o.decompose(&task))
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

// ========== SubagentPool 接入（v1.1 M10.5 Task C4）==========

#[tokio::test]
async fn orchestrator_uses_subagent_pool() {
    // LlmRouter 用 make_test_router（placeholder 是 #[cfg(test)] 私有，外部 tests 调不到）
    let router = make_test_router();
    let pool = std::sync::Arc::new(SubagentPool::start(2));
    let events = EventChannel::new();
    let orch = Orchestrator::with_pool(
        router,
        std::sync::Arc::new(ToolRegistry::new()),
        events,
        pool,
    );
    // v1.1 验证：with_pool 构造不 panic
    let _ = orch;
}

// ========== v1.2 E3: compute_step_layers 把步骤按依赖分层 ==========

#[test]
fn orchestrator_compute_step_layers_groups_by_dependency() {
    use eflow::application::orchestrator::Orchestrator;
    use eflow::common::types::{PlannedStep, TaskPlan};
    use uuid::Uuid;

    // 构造一个 5 步骤计划：
    //   step 0 (无依赖)        → layer 0
    //   step 1 (无依赖)        → layer 0
    //   step 2 (depends on 0)  → layer 1
    //   step 3 (depends on 1)  → layer 1
    //   step 4 (depends on 2)  → layer 2
    let plan = TaskPlan {
        task_id: Uuid::new_v4(),
        steps: vec![
            PlannedStep {
                order: 0,
                action: "a".into(),
                tool: "llm".into(),
                params: serde_json::json!({}),
                depends_on: None,
            },
            PlannedStep {
                order: 1,
                action: "b".into(),
                tool: "llm".into(),
                params: serde_json::json!({}),
                depends_on: None,
            },
            PlannedStep {
                order: 2,
                action: "c".into(),
                tool: "llm".into(),
                params: serde_json::json!({}),
                depends_on: Some(0),
            },
            PlannedStep {
                order: 3,
                action: "d".into(),
                tool: "llm".into(),
                params: serde_json::json!({}),
                depends_on: Some(1),
            },
            PlannedStep {
                order: 4,
                action: "e".into(),
                tool: "llm".into(),
                params: serde_json::json!({}),
                depends_on: Some(2),
            },
        ],
        estimated_steps: 5,
        risk_level: RiskLevel::L0,
    };

    let layers = Orchestrator::compute_step_layers(&plan);
    assert_eq!(
        layers.len(),
        3,
        "should be 3 layers: layer0=[0,1] layer1=[2,3] layer2=[4]"
    );
    assert_eq!(layers[0], vec![0, 1]);
    assert_eq!(layers[1], vec![2, 3]);
    assert_eq!(layers[2], vec![4]);
}

// ========== v1.2 E4: 步骤按层并行派发 ==========
// plan §E4 step 1 标注 E4 的"真"测试在 E6（pool_test 集成测试），
// 此处只保留一个轻量 witness 防止 Orchestrator::compute_step_layers 被改签名。
// 验证：函数指针 + 单 step 计划（1 步走 1 层）→ 1 层含 1 步骤。

#[test]
fn orchestrator_parallel_dispatch_witness_single_layer() {
    use eflow::application::orchestrator::Orchestrator;
    use eflow::common::types::{PlannedStep, TaskPlan};
    use uuid::Uuid;

    let plan = TaskPlan {
        task_id: Uuid::new_v4(),
        steps: vec![PlannedStep {
            order: 0,
            action: "only".into(),
            tool: "llm".into(),
            params: serde_json::json!({}),
            depends_on: None,
        }],
        estimated_steps: 1,
        risk_level: RiskLevel::L0,
    };
    let layers = Orchestrator::compute_step_layers(&plan);
    assert_eq!(layers.len(), 1, "单步计划应只产生 1 层");
    assert_eq!(layers[0], vec![0], "该层只含 order=0 这一步");
}
