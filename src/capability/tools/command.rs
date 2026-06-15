use async_trait::async_trait;

use super::registry::{Tool, ToolDefinition, ToolOutput};
use crate::common::error::{EflowError, Result};
use crate::common::types::RiskLevel;
use rust_i18n::t;

pub struct ExecuteCommandTool;

#[async_trait]
impl Tool for ExecuteCommandTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "execute_command".into(),
            description: "执行系统命令并返回输出".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "要执行的命令"},
                    "args": {"type": "array", "items": {"type": "string"}, "description": "命令参数"}
                },
                "required": ["command"]
            }),
            risk_level: RiskLevel::L2,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput> {
        let command = params["command"].as_str().ok_or_else(|| {
            EflowError::Tool(t!("err_tool_missing_param", name = "command").to_string())
        })?;

        let args: Vec<String> = params["args"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let output = tokio::process::Command::new(command)
            .args(&args)
            .output()
            .await
            .map_err(|e| {
                EflowError::Tool(
                    t!("err_tool_execute", command = command, msg = e.to_string()).to_string(),
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(ToolOutput {
            success: output.status.success(),
            content: if stdout.is_empty() {
                stderr.clone()
            } else {
                stdout
            },
            metadata: Some(serde_json::json!({
                "exit_code": output.status.code(),
                "stderr": stderr,
            })),
        })
    }
}
