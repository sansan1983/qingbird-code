use async_trait::async_trait;
use std::path::Path;

use crate::registry::{Tool, ToolDefinition, ToolOutput};
use qbird_code_models::{EflowError, Result, RiskLevel};
use rust_i18n::t;

const MAX_ENTRIES: usize = 1000;

pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_dir".into(),
            description: t!("tool_list_dir_description").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "目录路径"}
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

        let dir = Path::new(path);
        if !dir.exists() {
            return Err(EflowError::Tool(
                t!("err_tool_invalid_path", path = path).to_string(),
            ));
        }
        if !dir.is_dir() {
            return Err(EflowError::Tool(
                t!("err_tool_not_a_directory", path = path).to_string(),
            ));
        }

        let mut entries: Vec<String> = Vec::new();
        let mut total = 0usize;
        let mut truncated = false;

        let mut read_dir = tokio::fs::read_dir(dir).await.map_err(|e| {
            EflowError::Tool(t!("err_tool_read_dir", path = path, msg = e.to_string()).to_string())
        })?;

        while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
            EflowError::Tool(t!("err_tool_read_dir", path = path, msg = e.to_string()).to_string())
        })? {
            total += 1;
            if entries.len() >= MAX_ENTRIES {
                truncated = true;
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            let ft = entry.file_type().await.ok();
            let prefix = match ft {
                Some(t) if t.is_dir() => "[DIR]",
                Some(t) if t.is_symlink() => "[LINK]",
                _ => "[FILE]",
            };
            entries.push(format!("{} {}", prefix, name));
        }

        entries.sort();

        let count = entries.len();
        let body = t!(
            "tool_list_dir_result",
            path = path,
            count = count,
            total = total
        )
        .to_string()
            + "\n"
            + &entries.join("\n");

        Ok(ToolOutput {
            success: true,
            content: body,
            metadata: Some(serde_json::json!({
                "path": path,
                "entries": count,
                "total": total,
                "truncated": truncated,
            })),
        })
    }
}
