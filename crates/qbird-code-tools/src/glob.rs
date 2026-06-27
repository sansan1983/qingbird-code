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
        let mut truncated = false;
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
            let rel_str = rel.to_string_lossy();
            let normalized = rel_str.replace("\\", "/");
            if glob_match(pattern, &normalized) {
                results.push(rel.display().to_string());
                if results.len() >= MAX_RESULTS {
                    truncated = true;
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
            metadata: Some(serde_json::json!({"matches": count, "truncated": truncated})),
        })
    }
}

pub fn glob_match(pattern: &str, path: &str) -> bool {
    let mut re = String::with_capacity(pattern.len() + 2);
    re.push('^');
    let mut it = pattern.chars().peekable();

    while let Some(c) = it.next() {
        match c {
            '*' => {
                if it.peek() == Some(&'*') {
                    it.next();
                    re.push_str(".*");
                } else {
                    re.push_str("[^/]*");
                }
            }
            '?' => {
                re.push('.');
            }
            '[' => {
                re.push('[');
                if it.peek() == Some(&'!') {
                    it.next();
                    re.push('^');
                }
                if it.peek() == Some(&']') {
                    it.next();
                    re.push(']');
                }
                while let Some(&n) = it.peek() {
                    if n == ']' {
                        break;
                    }
                    re.push(n);
                    it.next();
                }
                if it.peek() == Some(&']') {
                    it.next();
                    re.push(']');
                }
            }
            _ => {
                if ".() {}+^$|\\".contains(c) {
                    re.push('\\');
                }
                re.push(c);
            }
        }
    }
    re.push('$');

    match regex_lite::Regex::new(&re) {
        Ok(r) => r.is_match(path),
        Err(e) => {
            tracing::warn!("invalid glob pattern {:?}: {}", pattern, e);
            false
        }
    }
}
