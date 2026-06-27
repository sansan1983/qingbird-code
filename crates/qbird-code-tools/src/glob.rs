use async_trait::async_trait;
use std::path::Path;

use crate::registry::{Tool, ToolDefinition, ToolOutput};
use qbird_code_models::{EflowError, Result, RiskLevel};
use rust_i18n::t;

const MAX_RESULTS: usize = 200;

pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "glob".into(),
            description: t!("tool_glob_description").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "glob 匹配模式，如 **/*.rs"},
                    "path": {"type": "string", "description": "搜索根目录，默认 '.'"}
                },
                "required": ["pattern"]
            }),
            risk_level: RiskLevel::L0,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput> {
        let pattern = params["pattern"].as_str().ok_or_else(|| {
            EflowError::Tool(t!("err_tool_missing_param", name = "pattern").to_string())
        })?;
        let root = params["path"].as_str().unwrap_or(".");

        let base = Path::new(root);
        if !base.exists() {
            return Err(EflowError::Tool(
                t!("err_tool_invalid_path", path = root).to_string(),
            ));
        }

        let mut results: Vec<String> = Vec::new();
        for entry in walkdir::WalkDir::new(base)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let rel = path.strip_prefix(base).unwrap_or(path);
            if glob_match(pattern, rel.to_str().unwrap_or("")) {
                results.push(rel.display().to_string());
                if results.len() >= MAX_RESULTS {
                    break;
                }
            }
        }

        let count = results.len();
        Ok(ToolOutput {
            success: true,
            content: if results.is_empty() {
                t!("tool_glob_no_match", pattern = pattern).to_string()
            } else {
                t!("tool_glob_count", count = count).to_string() + "\n" + &results.join("\n")
            },
            metadata: Some(serde_json::json!({"matches": count})),
        })
    }
}

fn glob_match(pattern: &str, path: &str) -> bool {
    let regex_str = pattern
        .replace(".", "\\.")
        .replace("**", "\0")
        .replace("*", "[^/]*")
        .replace("\0", ".*")
        .replace("?", ".");
    let re = regex_lite::Regex::new(&format!("^{}$", regex_str)).ok();
    re.map(|r| r.is_match(path)).unwrap_or(false)
}
