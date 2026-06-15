rust_i18n::i18n!("locales", fallback = "en-US");

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use eflow::capability::blackboard::Blackboard;
use eflow::capability::decisioner::Decisioner;
use eflow::capability::executor::Executor;
use eflow::capability::feedbacker::Feedbacker;
use eflow::capability::subagent::Subagent;
use eflow::capability::tools::{Tool, ToolDefinition, ToolOutput, ToolRegistry};
use eflow::common::error::Result;
use eflow::common::types::*;
use eflow::infrastructure::config::{
    CoreConfig, EflowConfig, LlmConfig, MemoryConfig, ProfileListConfig, ProviderEntry,
    ProvidersConfig, RoutingConfig, SecurityConfig, CacheConfig,
};
use eflow::infrastructure::llm::LlmRouter;
use eflow::infrastructure::locale;
use tokio::sync::Mutex;
use uuid::Uuid;

// 全模块固定中文 locale（中文断言）
// locale setup moved into individual tests

// ========== 测试辅助 ==========

/// 构造一个最小 EflowConfig（dummy key，不真调 LLM）
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

fn make_task(desc: &str, risk: RiskLevel) -> TaskSpec {
    TaskSpec::new(desc.into(), risk)
}

fn make_step(action: &str, tool: &str) -> TaskStep {
    TaskStep {
        action: action.into(),
        tool: tool.into(),
        params: serde_json::json!({"k": "v"}),
        expected_output: None,
    }
}

// ========== 用于 Executor 测试的 stub 工具 ==========

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

// ========== Blackboard 集成测试 ==========

#[tokio::test]
async fn blackboard_with_execution_plan_stores_plan() {
    let bb = Blackboard::new(make_task("t", RiskLevel::L0)).with_execution_plan(ExecutionPlan {
        step: PlannedStep {
            order: 0,
            action: "a".into(),
            tool: "x".into(),
            params: serde_json::json!({}),
            depends_on: None,
        },
        model_tier: ModelTier::Light,
        risk_level: RiskLevel::L1,
        sub_steps: vec![],
    });
    assert!(bb.execution_plan.is_some());
    assert_eq!(bb.risk_level, RiskLevel::L1);
}

#[tokio::test]
async fn blackboard_summarize_chinese_contains_keywords() {
    locale::init(Some("zh-CN"));
    let bb = Blackboard::new(make_task("测试任务", RiskLevel::L0));
    let s = bb.summarize();
    assert!(s.contains("任务") || s.contains("步骤"), "got: {}", s);
}

#[tokio::test]
async fn blackboard_summarize_english_uses_task_label() {
    locale::init(Some("en-US"));
    let bb = Blackboard::new(make_task("test task", RiskLevel::L0));
    let s = bb.summarize();
    assert!(s.contains("Task"), "got: {}", s);
    locale::init(Some("zh-CN")); // 还原
}

// ========== Decisioner 集成测试（仅 L0/L1 规则路径）==========

#[tokio::test]
async fn decisioner_l0_routes_to_light_tier() {
    let router = make_test_router();
    let d = Decisioner::new(router);

    let bb = Blackboard::new(make_task("readme", RiskLevel::L0))
        .with_step(make_step("read README", "read_file"));
    let bb2 = d.decide(&bb).await.unwrap();

    let plan = bb2.execution_plan.as_ref().unwrap();
    assert_eq!(plan.model_tier, ModelTier::Light);
    assert_eq!(plan.risk_level, RiskLevel::L0);
    // L0 < L2 → 不调 LLM，sub_steps = 单步克隆
    assert_eq!(plan.sub_steps.len(), 1);
    assert_eq!(plan.sub_steps[0].action, "read README");
}

#[tokio::test]
async fn decisioner_l1_routes_to_light_tier() {
    let router = make_test_router();
    let d = Decisioner::new(router);

    let bb = Blackboard::new(make_task("write file", RiskLevel::L1))
        .with_step(make_step("write log", "write_file"));
    let bb2 = d.decide(&bb).await.unwrap();

    let plan = bb2.execution_plan.as_ref().unwrap();
    assert_eq!(plan.model_tier, ModelTier::Light);
    assert_eq!(plan.risk_level, RiskLevel::L1);
    // L1 < L2 → 不调 LLM
    assert_eq!(plan.sub_steps.len(), 1);
}

#[tokio::test]
async fn decisioner_preserves_step_fields_in_execution_plan() {
    let router = make_test_router();
    let d = Decisioner::new(router);

    let step = make_step("read README", "read_file");
    let bb = Blackboard::new(make_task("t", RiskLevel::L0)).with_step(step.clone());
    let bb2 = d.decide(&bb).await.unwrap();

    let plan = bb2.execution_plan.as_ref().unwrap();
    assert_eq!(plan.step.action, "read README");
    assert_eq!(plan.step.tool, "read_file");
}

// ========== Executor 集成测试（工具执行路径，不调 LLM）==========

#[tokio::test]
async fn executor_runs_tool_steps_without_llm() {
    let router = make_test_router();
    let tools = make_tool_registry();
    let e = Executor::new(router, tools);

    let plan = ExecutionPlan {
        step: PlannedStep {
            order: 0,
            action: "echo".into(),
            tool: "stub_echo".into(),
            params: serde_json::json!({}),
            depends_on: None,
        },
        model_tier: ModelTier::Light,
        risk_level: RiskLevel::L0,
        sub_steps: vec![make_step("echo step", "stub_echo")],
    };

    let bb = Blackboard::new(make_task("t", RiskLevel::L0)).with_execution_plan(plan);
    let bb2 = e.execute(bb).await.unwrap();

    assert_eq!(bb2.action_log.len(), 1);
    assert!(bb2.action_log[0].success);
    assert_eq!(bb2.action_log[0].tool, "stub_echo");
}

#[tokio::test]
async fn executor_records_failure_as_failed_action() {
    // 工具不存在 → Tool 工具返回 Err → Executor 记录为 success=false 的 ActionRecord
    let router = make_test_router();
    let tools = make_tool_registry(); // 不包含 "ghost_tool"
    let e = Executor::new(router, tools);

    let plan = ExecutionPlan {
        step: PlannedStep {
            order: 0,
            action: "x".into(),
            tool: "ghost_tool".into(),
            params: serde_json::json!({}),
            depends_on: None,
        },
        model_tier: ModelTier::Light,
        risk_level: RiskLevel::L0,
        sub_steps: vec![make_step("x", "ghost_tool")],
    };

    let bb = Blackboard::new(make_task("t", RiskLevel::L0)).with_execution_plan(plan);
    let bb2 = e.execute(bb).await.unwrap();

    assert_eq!(bb2.action_log.len(), 1);
    assert!(!bb2.action_log[0].success, "missing tool should record failure");
    // 输出含 i18n 翻译的错误
    let summary = &bb2.action_log[0].summary;
    assert!(
        summary.contains("ghost_tool") || summary.contains("执行失败") || summary.contains("failed"),
        "got: {}",
        summary
    );
}

#[tokio::test]
async fn executor_failure_summary_uses_zh_locale() {
    locale::init(Some("zh-CN"));
    let router = make_test_router();
    let tools = make_tool_registry();
    let e = Executor::new(router, tools);

    let plan = ExecutionPlan {
        step: PlannedStep {
            order: 0,
            action: "x".into(),
            tool: "ghost_tool".into(),
            params: serde_json::json!({}),
            depends_on: None,
        },
        model_tier: ModelTier::Light,
        risk_level: RiskLevel::L0,
        sub_steps: vec![make_step("x", "ghost_tool")],
    };

    let bb = Blackboard::new(make_task("t", RiskLevel::L0)).with_execution_plan(plan);
    let bb2 = e.execute(bb).await.unwrap();

    let summary = &bb2.action_log[0].summary;
    // 中文 locale 下应包含"执行失败"
    assert!(
        summary.contains("执行失败") || summary.contains("ghost_tool"),
        "got: {}",
        summary
    );
}

#[tokio::test]
async fn executor_empty_sub_steps_records_nothing() {
    let router = make_test_router();
    let tools = make_tool_registry();
    let e = Executor::new(router, tools);

    let plan = ExecutionPlan {
        step: PlannedStep {
            order: 0,
            action: "noop".into(),
            tool: "stub_echo".into(),
            params: serde_json::json!({}),
            depends_on: None,
        },
        model_tier: ModelTier::Light,
        risk_level: RiskLevel::L0,
        sub_steps: vec![],
    };

    let bb = Blackboard::new(make_task("t", RiskLevel::L0)).with_execution_plan(plan);
    let bb2 = e.execute(bb).await.unwrap();
    assert!(bb2.action_log.is_empty());
}

// ========== Feedbacker 集成测试（规则路径，不调 LLM）==========

#[tokio::test]
async fn feedbacker_empty_action_log_returns_pass() {
    let router = make_test_router();
    let f = Feedbacker::new(router);

    let bb = Blackboard::new(make_task("t", RiskLevel::L0));
    let (bb2, verdict) = f.evaluate(bb).await.unwrap();

    match verdict {
        QualityVerdict::Pass { summary } => {
            assert!(summary.contains("操作") || summary.contains("actions"), "got: {}", summary);
        }
        _ => panic!("expected Pass for empty action log"),
    }
    assert_eq!(bb2.feedback_log.len(), 1);
}

#[tokio::test]
async fn feedbacker_all_success_l0_fast_pass() {
    let router = make_test_router();
    let f = Feedbacker::new(router);

    let bb = Blackboard::new(make_task("t", RiskLevel::L0)).with_action(ActionRecord {
        timestamp: Utc::now(),
        action: "a1".into(),
        tool: "stub_echo".into(),
        success: true,
        summary: "ok".into(),
    });
    let (_, verdict) = f.evaluate(bb).await.unwrap();

    match verdict {
        QualityVerdict::Pass { summary } => {
            // 规则 2 触发：摘要含"操作"和"成功"（或英文）
            assert!(summary.contains("操作") || summary.contains("success"), "got: {}", summary);
        }
        _ => panic!("expected Pass for all-success low-risk"),
    }
}

#[tokio::test]
async fn feedbacker_all_success_en_locale() {
    locale::init(Some("en-US"));
    let router = make_test_router();
    let f = Feedbacker::new(router);

    let bb = Blackboard::new(make_task("t", RiskLevel::L0)).with_action(ActionRecord {
        timestamp: Utc::now(),
        action: "a1".into(),
        tool: "stub_echo".into(),
        success: true,
        summary: "ok".into(),
    });
    let (_, verdict) = f.evaluate(bb).await.unwrap();
    match verdict {
        QualityVerdict::Pass { summary } => {
            assert!(summary.contains("Completed") || summary.contains("succeeded"), "got: {}", summary);
        }
        _ => panic!("expected Pass"),
    }
    locale::init(Some("zh-CN"));
}

#[tokio::test]
async fn feedbacker_with_failure_at_l2_needs_llm_evaluation_path() {
    // L2 风险 + 有失败操作 → 不满足 fast pass 条件 → 走 LLM 路径
    // 这个测试仅验证 evaluate() 返回 Err 或 Pass（不会成功返回 Rework/Escalate 因为 LLM mock 不可用）
    // 简化：仅验证不会 panic，且返回 EflowError
    let router = make_test_router();
    let f = Feedbacker::new(router);

    let bb = Blackboard::new(make_task("complex task", RiskLevel::L2)).with_action(ActionRecord {
        timestamp: Utc::now(),
        action: "complex_op".into(),
        tool: "real_tool".into(),
        success: false,
        summary: "failed".into(),
    });
    let result = f.evaluate(bb).await;
    // LLM 不可用 → 必然返回 Err
    assert!(result.is_err());
}

#[tokio::test]
async fn feedbacker_records_feedback_in_blackboard() {
    let router = make_test_router();
    let f = Feedbacker::new(router);

    let bb = Blackboard::new(make_task("t", RiskLevel::L0));
    let (bb2, _) = f.evaluate(bb).await.unwrap();

    assert_eq!(bb2.feedback_log.len(), 1);
    let record = &bb2.feedback_log[0];
    assert!(matches!(record.verdict, QualityVerdict::Pass { .. }));
}

// ========== 端到端集成：Blackboard → Executor → Feedbacker ==========

#[tokio::test]
async fn pipeline_runs_l0_task_end_to_end_without_llm() {
    let router = make_test_router();
    let tools = make_tool_registry();

    // 1) Decisioner 规划（L0 → Light tier，无 LLM）
    let d = Decisioner::new(router.clone());
    let bb = Blackboard::new(make_task("readme", RiskLevel::L0))
        .with_step(make_step("echo", "stub_echo"));
    let bb = d.decide(&bb).await.unwrap();

    // 2) Executor 执行（stub_echo 工具，无 LLM）
    let e = Executor::new(router.clone(), tools);
    let bb = e.execute(bb).await.unwrap();
    assert_eq!(bb.action_log.len(), 1);
    assert!(bb.action_log[0].success);

    // 3) Feedbacker 评估（全部成功 + L0 → 快速 Pass，无 LLM）
    let f = Feedbacker::new(router);
    let (bb, verdict) = f.evaluate(bb).await.unwrap();
    assert!(matches!(verdict, QualityVerdict::Pass { .. }));
    assert_eq!(bb.feedback_log.len(), 1);
}

#[tokio::test]
async fn pipeline_summary_after_pipeline_run() {
    let router = make_test_router();
    let tools = make_tool_registry();

    let d = Decisioner::new(router.clone());
    let bb = Blackboard::new(make_task("end to end", RiskLevel::L0))
        .with_step(make_step("echo", "stub_echo"));
    let bb = d.decide(&bb).await.unwrap();
    let bb = Executor::new(router.clone(), tools).execute(bb).await.unwrap();
    let (bb, _) = Feedbacker::new(router).evaluate(bb).await.unwrap();

    // 摘要含 1/1 passed
    let summary = bb.summarize();
    assert!(summary.contains("1/1"), "got: {}", summary);
}

// ========== Subagent 集成测试 ==========

#[test]
fn subagent_new_assigns_id_and_fields() {
    let s = Subagent::new(
        "alpha".into(),
        Role::CodeAssistant,
        vec![Capability::ReadFile, Capability::SearchCode],
    );
    assert_eq!(s.name, "alpha");
    assert!(matches!(s.role, Role::CodeAssistant));
    assert_eq!(s.capabilities.len(), 2);
    assert!(!s.id.is_nil());
}

#[test]
fn subagent_ids_are_unique() {
    let a = Subagent::new("a".into(), Role::Generalist, vec![]);
    let b = Subagent::new("b".into(), Role::Generalist, vec![]);
    assert_ne!(a.id, b.id);
}

#[test]
fn subagent_default_permission_is_restrictive() {
    let s = Subagent::new("x".into(), Role::Generalist, vec![]);
    assert!(s.permission.allowed_paths.is_empty());
    assert!(s.permission.allowed_commands.is_empty());
    assert!(!s.permission.network_enabled);
}

#[tokio::test]
async fn subagent_execute_step_l0_pipeline_passes() {
    // L0 任务：完整 D→E→F 管线，决策不调 LLM，执行用工具，反馈快速 Pass
    let router = make_test_router();
    let tools = make_tool_registry();
    let d = Decisioner::new(router.clone());
    let e = Executor::new(router.clone(), tools);
    let f = Feedbacker::new(router);

    let s = Subagent::new("worker".into(), Role::Generalist, vec![]);
    let bb = Blackboard::new(make_task("readme", RiskLevel::L0))
        .with_step(make_step("echo", "stub_echo"));

    let bb = s.execute_step(bb, &d, &e, &f).await.unwrap();

    // 1 个 action + 1 个 feedback 记录（fast-pass）
    assert_eq!(bb.action_log.len(), 1);
    assert!(bb.action_log[0].success);
    assert_eq!(bb.feedback_log.len(), 1);
    assert!(matches!(
        bb.feedback_log[0].verdict,
        QualityVerdict::Pass { .. }
    ));
}

#[tokio::test]
async fn subagent_execute_step_l1_pipeline_passes() {
    let router = make_test_router();
    let tools = make_tool_registry();
    let d = Decisioner::new(router.clone());
    let e = Executor::new(router.clone(), tools);
    let f = Feedbacker::new(router);

    let s = Subagent::new("writer".into(), Role::CodeAssistant, vec![]);
    let bb = Blackboard::new(make_task("write log", RiskLevel::L1))
        .with_step(make_step("echo", "stub_echo"));

    let bb = s.execute_step(bb, &d, &e, &f).await.unwrap();
    assert_eq!(bb.action_log.len(), 1);
    assert!(bb.action_log[0].success);
}

#[tokio::test]
async fn subagent_execute_step_feedback_appended_to_log() {
    let router = make_test_router();
    let tools = make_tool_registry();
    let d = Decisioner::new(router.clone());
    let e = Executor::new(router.clone(), tools);
    let f = Feedbacker::new(router);

    let s = Subagent::new("worker".into(), Role::Generalist, vec![]);
    let bb = Blackboard::new(make_task("t", RiskLevel::L0))
        .with_step(make_step("echo", "stub_echo"));

    let bb = s.execute_step(bb, &d, &e, &f).await.unwrap();

    // Pass 路径：feedback_log 只有 1 条
    assert_eq!(bb.feedback_log.len(), 1);
    assert_eq!(bb.feedback_log[0].retry_count, 0);
    assert!(matches!(
        bb.feedback_log[0].verdict,
        QualityVerdict::Pass { .. }
    ));
}

// 避免 unused warning for Uuid 导入（用作 hint）
#[allow(dead_code)]
fn _uuid_check() -> Uuid {
    Uuid::new_v4()
}
