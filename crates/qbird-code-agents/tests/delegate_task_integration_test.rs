use std::sync::Arc;

use async_trait::async_trait;
use qbird_code_agents::delegate_task::DelegateTaskTool;
use qbird_code_agents::subagent::{
    ChildRecord, ChildStatus, SubagentExecutorTrait, SubagentSpawnHints, ToolPolicy,
};
use qbird_code_models::{EflowError, Result};
use qbird_code_tools::ToolRegistry;

struct MockExecutor;

#[async_trait]
impl SubagentExecutorTrait for MockExecutor {
    fn list_profile_names(&self) -> Vec<String> {
        vec!["general".into(), "explore".into()]
    }
    fn validate_profile(&self, name: &str) -> Result<()> {
        if name == "general" || name == "explore" {
            Ok(())
        } else {
            Err(EflowError::SubagentProfileNotFound { name: name.into() })
        }
    }
    async fn spawn_child_with_provider(
        &self,
        profile_name: &str,
        _prompt: &str,
        _hints: &SubagentSpawnHints,
        _provider: &dyn qbird_code_infra::providers::Provider,
        _http: &qbird_code_infra::http_client::HttpLlmClient,
    ) -> Result<ChildRecord> {
        Ok(ChildRecord {
            child_id: "mock-child".into(),
            status: ChildStatus::Completed,
            summary: format!("mock result for profile={}", profile_name),
            usage: Default::default(),
            profile: profile_name.into(),
            tool_policy: ToolPolicy::Inherit,
            duration_ms: 42,
        })
    }
}

#[test]
fn delegate_task_tool_appears_in_registry_definitions() {
    let mut registry = ToolRegistry::new();
    let mock: Arc<dyn SubagentExecutorTrait> = Arc::new(MockExecutor);
    let tool = DelegateTaskTool::new(mock);
    registry.register(Arc::new(tool));

    let defs = registry.definitions();
    let delegate_def = defs.iter().find(|d| d.name == "delegate_task");
    assert!(delegate_def.is_some(), "delegate_task must be in registry");
    let def = delegate_def.unwrap();
    assert!(
        def.description.contains("general"),
        "description must list 'general' profile; got: {}",
        def.description
    );
    assert!(
        def.description.contains("explore"),
        "description must list 'explore' profile; got: {}",
        def.description
    );
}

#[tokio::test]
async fn delegate_task_tool_in_registry_executes_via_registry() {
    let mut registry = ToolRegistry::new();
    let mock: Arc<dyn SubagentExecutorTrait> = Arc::new(MockExecutor);
    let tool = DelegateTaskTool::new(mock);
    registry.register(Arc::new(tool));

    // delegate_task 不能走 ToolRegistry.execute (需要 provider/http_client)，
    // 但 Tool trait 默认的 execute 应该返回明确错误指引调用方走 execute_with_provider
    let result = registry
        .execute(
            "delegate_task",
            serde_json::json!({
                "label": "integration-test",
                "prompt": "say hi",
                "profile": "explore",
            }),
            uuid::Uuid::new_v4(),
        )
        .await;

    let err = result.expect_err("delegate_task 应拒绝走 ToolRegistry.execute，要求 provider 注入");
    let msg = format!("{}", err);
    assert!(
        msg.contains("execute_with_provider") || msg.contains("provider"),
        "错误应指明走 execute_with_provider 路径；got: {}",
        msg
    );
}
