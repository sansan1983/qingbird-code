use async_trait::async_trait;
use std::path::Path;
use std::sync::{Arc, Mutex};

use similar::TextDiff;

use crate::registry::{Tool, ToolDefinition, ToolOutput};
use crate::undo_stack::UndoStack;
use qbird_code_models::{EflowError, Result, RiskLevel};
use rust_i18n::t;

#[derive(Default)]
pub struct EditTool {
    undo_stack: Option<Arc<Mutex<UndoStack>>>,
}

impl EditTool {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach an undo stack. On successful edit, the original file content
    /// is pushed onto this stack before writing the replacement.
    pub fn with_undo_stack(mut self, stack: Arc<Mutex<UndoStack>>) -> Self {
        self.undo_stack = Some(stack);
        self
    }
}

/// Trim a search string into a stable 80-char excerpt suitable for embedding
/// in error messages and the diff summary.
fn search_excerpt(s: &str) -> String {
    const MAX: usize = 80;
    let trimmed = s.trim();
    if trimmed.chars().count() <= MAX {
        trimmed.to_string()
    } else {
        let truncated: String = trimmed.chars().take(MAX).collect();
        format!("{truncated}…")
    }
}

/// Returns the 1-based line numbers on which `needle` appears as a substring
/// of `haystack`. Returns an empty vec if `needle` is empty.
///
/// Counts every overlap. For a needle that appears twice on the same line,
/// both hits are reported as the same line number (caller de-dupes if needed
/// for display, but we keep them separate so the count stays accurate).
fn match_lines(haystack: &str, needle: &str) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    let mut line: usize = 1;
    let mut search_start = 0usize;
    while let Some(rel) = haystack[search_start..].find(needle) {
        let abs = search_start + rel;
        // Count newlines between previous search_start and abs to find the
        // 1-based line number containing this hit.
        let newlines = haystack[search_start..abs].matches('\n').count();
        line += newlines;
        hits.push(line);
        // Advance past the hit. If needle ends with a '\n' the next line
        // counter must step by one too — but replacen would never match
        // across line boundaries for trailing newline, so this is safe.
        search_start = abs + needle.len();
        // Re-sync line counter to the position just after the hit.
        let total_newlines = haystack[..search_start].matches('\n').count();
        line = total_newlines + 1;
    }
    hits
}

#[async_trait]
impl Tool for EditTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "edit".into(),
            description: "精确替换文件中的一段文本".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "文件路径"},
                    "old_string": {"type": "string", "description": "要替换的原文（必须恰好出现 1 次）"},
                    "new_string": {"type": "string", "description": "替换后的文本"}
                },
                "required": ["path", "old_string", "new_string"]
            }),
            risk_level: RiskLevel::L1,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput> {
        let path = params["path"].as_str().ok_or_else(|| {
            EflowError::Tool(t!("err_tool_missing_param", name = "path").to_string())
        })?;
        let old_string = params["old_string"].as_str().ok_or_else(|| {
            EflowError::Tool(t!("err_tool_missing_param", name = "old_string").to_string())
        })?;
        let new_string = params["new_string"].as_str().ok_or_else(|| {
            EflowError::Tool(t!("err_tool_missing_param", name = "new_string").to_string())
        })?;

        let path_buf = Path::new(path);

        // Read current content. A missing file is mapped to
        // `ToolEditNotFound` so the LLM receives a clear "not found" signal
        // (the most likely cause when the search string is also absent).
        let content = match tokio::fs::read_to_string(path_buf).await {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(EflowError::ToolEditNotFound {
                    path: path.to_string(),
                    search_excerpt: search_excerpt(old_string),
                });
            }
            Err(e) => return Err(e.into()),
        };

        // Substring search for old_string. Returns 1-based line numbers
        // (duplicates on the same line each occupy an entry).
        let line_numbers = match_lines(&content, old_string);
        let count = line_numbers.len();

        if count == 0 {
            return Err(EflowError::ToolEditNotFound {
                path: path.to_string(),
                search_excerpt: search_excerpt(old_string),
            });
        }
        if count > 1 {
            return Err(EflowError::ToolEditAmbiguous {
                path: path.to_string(),
                count,
                line_numbers,
                search_excerpt: search_excerpt(old_string),
            });
        }

        // Apply the single replacement.
        let new_content = content.replacen(old_string, new_string, 1);

        // Compute a line-level diff for the summary.
        let diff = TextDiff::from_lines(&content, &new_content);
        let insert_count = diff
            .iter_all_changes()
            .filter(|c| c.tag() == similar::ChangeTag::Insert)
            .count();
        let delete_count = diff
            .iter_all_changes()
            .filter(|c| c.tag() == similar::ChangeTag::Delete)
            .count();
        let delta = insert_count as i64 - delete_count as i64;

        // Push original content onto the undo stack before writing.
        if let Some(ref stack) = self.undo_stack
            && let Ok(mut s) = stack.lock()
        {
            s.push(path_buf.to_path_buf(), content.clone());
        }

        tokio::fs::write(path_buf, &new_content).await?;

        let old_line_count = content.lines().count();
        let new_line_count = new_content.lines().count();

        Ok(ToolOutput {
            success: true,
            content: t!(
                "interactive_edit_diff_summary",
                old_lines = old_line_count,
                new_lines = new_line_count,
                delta = delta
            )
            .to_string(),
            metadata: Some(serde_json::json!({
                "old_lines": old_line_count,
                "new_lines": new_line_count,
                "delta": delta,
            })),
        })
    }
}
