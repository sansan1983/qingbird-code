use async_trait::async_trait;
use std::path::Path;

use super::registry::{Tool, ToolDefinition, ToolOutput};
use crate::common::error::{EflowError, Result};
use crate::common::types::RiskLevel;
use rust_i18n::t;

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".into(),
            description: "读取指定文件的内容".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "文件路径"}
                },
                "required": ["path"]
            }),
            risk_level: RiskLevel::L0,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput> {
        let path = params["path"].as_str().ok_or_else(|| {
            EflowError::Tool(t!("err_tool_missing_param", name = "path").to_string())
        })?;

        let content = tokio::fs::read_to_string(Path::new(path))
            .await
            .map_err(|e| {
                EflowError::Tool(
                    t!("err_tool_read_file", path = path, msg = e.to_string()).to_string(),
                )
            })?;

        let line_count = content.lines().count();
        Ok(ToolOutput {
            success: true,
            content: t!("status_read_file_header", path = path, lines = line_count).to_string()
                + "\n\n"
                + &content,
            metadata: Some(serde_json::json!({"lines": line_count})),
        })
    }
}

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write_file".into(),
            description: "写入内容到指定文件".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "文件路径"},
                    "content": {"type": "string", "description": "写入内容"}
                },
                "required": ["path", "content"]
            }),
            risk_level: RiskLevel::L1,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput> {
        let path = params["path"].as_str().ok_or_else(|| {
            EflowError::Tool(t!("err_tool_missing_param", name = "path").to_string())
        })?;
        let content = params["content"].as_str().ok_or_else(|| {
            EflowError::Tool(t!("err_tool_missing_param", name = "content").to_string())
        })?;

        tokio::fs::write(Path::new(path), content)
            .await
            .map_err(|e| {
                EflowError::Tool(
                    t!("err_tool_write_file", path = path, msg = e.to_string()).to_string(),
                )
            })?;

        let bytes = content.len();
        Ok(ToolOutput {
            success: true,
            content: t!("status_written_bytes", path = path, bytes = bytes).to_string(),
            metadata: Some(serde_json::json!({"bytes_written": bytes})),
        })
    }
}
