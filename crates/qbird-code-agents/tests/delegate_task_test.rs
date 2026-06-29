use std::sync::Arc;

use async_trait::async_trait;
use qbird_code_agents::delegate_task::DelegateTaskTool;
use qbird_code_agents::subagent::{
    ChildRecord, ChildStatus, SubagentExecutorTrait, SubagentSpawnHints, ToolPolicy,
};
use qbird_code_models::{EflowError, Result};
use qbird_code_tools::Tool;
use serde_json::json;

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
            child_id: "test-child-id".into(),
            status: ChildStatus::Completed,
            summary: format!("mock done for {}", profile_name),
            usage: Default::default(),
            profile: profile_name.into(),
            tool_policy: ToolPolicy::Inherit,
            duration_ms: 100,
        })
    }
}

#[tokio::test]
async fn delegate_task_tool_definition_has_correct_schema() {
    let tool = DelegateTaskTool::new(Arc::new(MockExecutor));
    let def = tool.definition();
    assert_eq!(def.name, "delegate_task");
    assert!(
        def.description.contains("Available profiles") || def.description.contains("可用 profiles"),
        "description should mention profiles; got: {}",
        def.description
    );
    let props = def.parameters["properties"]
        .as_object()
        .expect("properties object");
    assert!(props.contains_key("label"));
    assert!(props.contains_key("prompt"));
    assert!(props.contains_key("profile"));
    assert!(props.contains_key("workspace"));
    assert!(props.contains_key("model"));
}

#[tokio::test]
async fn delegate_task_tool_plain_execute_returns_error() {
    let tool = DelegateTaskTool::new(Arc::new(MockExecutor));
    let result = tool.execute(json!({"label": "test"})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn delegate_task_tool_unknown_profile_returns_error() {
    let tool = DelegateTaskTool::new(Arc::new(MockExecutor));
    let result = tool.executor().validate_profile("nonexistent");
    assert!(matches!(
        result,
        Err(EflowError::SubagentProfileNotFound { .. })
    ));
}
