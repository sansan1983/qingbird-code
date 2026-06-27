use async_trait::async_trait;
use std::path::Path;

use regex_lite::Regex;
use walkdir::WalkDir;

use crate::glob::glob_match;
use crate::registry::{Tool, ToolDefinition, ToolOutput};
use qbird_code_models::{EflowError, Result, RiskLevel};
use rust_i18n::t;

const MAX_MATCHES: usize = 50;
const MAX_FILE_BYTES: u64 = 1_048_576; // 1 MiB — 跳过更大的文件，避免 OOM

pub struct SearchCodeTool;

#[async_trait]
impl Tool for SearchCodeTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "search_code".into(),
            description: "在代码仓库中按正则搜索文件内容（Rust 原生实现，跨平台）".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "搜索模式（支持正则）"},
                    "path": {"type": "string", "description": "搜索目录，默认 '.'"},
                    "file_types": {"type": "string", "description": "文件类型过滤，如 '*.rs,*.toml'，逗号分隔"}
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
        let search_path = params["path"].as_str().unwrap_or(".");
        let file_types = params["file_types"].as_str();

        // 解析 file_types 为 glob 列表
        let type_filters: Vec<String> = file_types
            .map(|s| {
                s.split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        // 编译正则
        let re = Regex::new(pattern).map_err(|e| {
            EflowError::Tool(
                t!(
                    "err_tool_invalid_regex",
                    pattern = pattern,
                    msg = e.to_string()
                )
                .to_string(),
            )
        })?;

        let root = Path::new(search_path);
        if !root.exists() {
            return Err(EflowError::Tool(
                t!("err_tool_invalid_path", path = search_path).to_string(),
            ));
        }

        // 收集匹配结果
        let mut hits: Vec<String> = Vec::new();
        let mut files_scanned: u32 = 0;

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();

            // file_types 过滤
            if !type_filters.is_empty() {
                let name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n,
                    None => continue,
                };
                if !type_filters.iter().any(|g| glob_match(g, name)) {
                    continue;
                }
            }

            // 跳过过大文件
            let size = match entry.metadata().map(|m| m.len()) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if size > MAX_FILE_BYTES {
                continue;
            }

            files_scanned += 1;

            let content = match tokio::fs::read_to_string(path).await {
                Ok(c) => c,
                Err(_) => continue, // 单文件失败跳过
            };

            for (i, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    hits.push(format!("{}:{}:{}", path.display(), i + 1, line));
                    if hits.len() >= MAX_MATCHES {
                        break;
                    }
                }
            }

            if hits.len() >= MAX_MATCHES {
                break;
            }
        }

        let count = hits.len();
        let body = if hits.is_empty() {
            t!("status_no_match", pattern = pattern).to_string()
        } else {
            t!("status_match_count", count = count).to_string() + "\n\n" + &hits.join("\n")
        };

        Ok(ToolOutput {
            success: true,
            content: body,
            metadata: Some(serde_json::json!({
                "matches": count,
                "files_scanned": files_scanned,
                "truncated": count >= MAX_MATCHES,
            })),
        })
    }
}
