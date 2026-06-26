use super::reference::ContextRef;
use crate::common::types::ActionRecord;
use rust_i18n::t;

/// Token 估算：每 X 字符 ≈ 1 token（fix v1.0.3 M5 抽离）
const CHARS_PER_TOKEN: usize = 4;
/// 上下文压缩触发阈值：当前 token 数超过 max 的 80% 即触发（fix v1.0.3 M5 抽离）
const COMPRESSION_THRESHOLD: f64 = 0.8;

/// 上下文压缩器
pub struct ContextCompressor;

impl ContextCompressor {
    /// L1 结构压缩：工具调用日志 → 摘要行
    #[must_use]
    pub fn compress_action_log(logs: &[ActionRecord]) -> String {
        if logs.is_empty() {
            return t!("ctx_no_actions").to_string();
        }

        let lines: Vec<String> = logs
            .iter()
            .map(|a| {
                let status = if a.success { "✓" } else { "✗" };
                let summary: String = a.summary.chars().take(100).collect();
                t!(
                    "ctx_action_log_line",
                    status = status,
                    time = a.timestamp.format("%H:%M:%S").to_string(),
                    tool = a.tool.clone(),
                    summary = summary
                )
                .to_string()
            })
            .collect();

        lines.join("\n")
    }

    /// L1 结构压缩：文件内容 → 路径 + 统计信息
    #[must_use]
    pub fn compress_file_content(path: &str, content: &str) -> (String, ContextRef) {
        let lines = content.lines().count();
        let bytes = content.len();
        let first_lines: Vec<&str> = content.lines().take(3).collect();
        let preview = first_lines.join("\n");

        let summary = t!(
            "ctx_file_summary",
            path = path,
            lines = lines,
            bytes = bytes
        )
        .to_string();
        let storage_key = format!("file:{path}");

        let token_cost = (bytes / CHARS_PER_TOKEN) as u32; // 粗略估算 token 数
        let ctx_ref = ContextRef::new(summary.clone(), storage_key, token_cost);

        (preview, ctx_ref)
    }

    /// L1 结构压缩：错误堆栈 → 错误类型 + 第一行
    #[must_use]
    pub fn compress_error(error: &str) -> String {
        let first_line = error.lines().next().unwrap_or("unknown error");
        t!("ctx_error_summary", msg = first_line).to_string()
    }

    /// L2 语义压缩（v1.0 简化版本 — 规则驱动摘要）
    /// v1.1 引入 LLM 驱动的语义压缩
    #[must_use]
    pub fn summarize_conversation(messages: &[String], max_summary_len: usize) -> String {
        if messages.len() <= 2 {
            return messages.join("\n");
        }

        // 规则驱动：保留首尾，中间截断
        let mut parts = vec![];
        parts.push(messages.first().cloned().unwrap_or_default());
        parts.push(
            t!(
                "ctx_conversation_omitted",
                count = messages.len().saturating_sub(2)
            )
            .to_string(),
        );
        parts.push(messages.last().cloned().unwrap_or_default());

        let summary = parts.join("\n");
        if summary.chars().count() > max_summary_len {
            // 按 char 切而非 byte 切，避免多字节 UTF-8 边界 panic（fix v1.0.3 B1）
            let truncated: String = summary.chars().take(max_summary_len).collect();
            format!("{truncated}...")
        } else {
            summary
        }
    }

    /// 估算 token 用量
    #[must_use]
    pub fn estimate_tokens(text: &str) -> u32 {
        (text.chars().count() as u32).div_ceil(CHARS_PER_TOKEN as u32)
    }

    /// 检查是否需要压缩
    #[must_use]
    pub fn needs_compression(current_tokens: u32, max_tokens: u32) -> bool {
        current_tokens > (f64::from(max_tokens) * COMPRESSION_THRESHOLD) as u32
    }
}
