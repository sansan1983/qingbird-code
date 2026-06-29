use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use qbird_code_models::{EflowError, RiskLevel};
use qbird_code_tools::{Tool, ToolDefinition, ToolOutput, ToolRegistry};

// ===== Stub tools for the whitelist test =====

struct StubTool {
    name: &'static str,
}

#[async_trait]
impl Tool for StubTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name.into(),
            description: "stub".into(),
            parameters: json!({"type": "object", "properties": {}}),
            risk_level: RiskLevel::L0,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> qbird_code_models::Result<ToolOutput> {
        Ok(ToolOutput {
            success: true,
            content: format!("executed {}", self.name),
            metadata: None,
        })
    }
}

// ===== Allowed tools whitelist =====

#[tokio::test]
async fn test_allowed_tools_blocks_unlisted() {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(StubTool { name: "read" }));
    reg.register(Arc::new(StubTool { name: "write" }));

    // Only allow "read" — `execute("write", ...)` must be rejected.
    reg.set_allowed_tools(Some(vec!["read".into()]));

    let res = reg.execute("write", json!({}), uuid::Uuid::new_v4()).await;
    match res {
        Err(EflowError::ToolNotAllowed { tool, allowed }) => {
            assert_eq!(tool, "write");
            assert_eq!(allowed, vec!["read".to_string()]);
        }
        other => panic!("expected ToolNotAllowed, got {other:?}"),
    }
}

#[tokio::test]
async fn test_allowed_tools_admits_listed() {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(StubTool { name: "read" }));
    reg.register(Arc::new(StubTool { name: "write" }));
    reg.set_allowed_tools(Some(vec!["read".into(), "write".into()]));

    let out = reg
        .execute("read", json!({}), uuid::Uuid::new_v4())
        .await
        .expect("read is allowed");
    assert!(out.success);
    assert!(out.content.contains("read"));
}

#[tokio::test]
async fn test_allowed_tools_none_allows_all() {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(StubTool { name: "read" }));
    reg.register(Arc::new(StubTool { name: "write" }));
    // No set_allowed_tools call — None default = all allowed.
    assert!(reg.allowed_tools().is_none());
    let out = reg
        .execute("write", json!({}), uuid::Uuid::new_v4())
        .await
        .expect("write is allowed when no whitelist");
    assert!(out.success);
}
