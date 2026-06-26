use qbird_code_models::Message;

/// Nudge 提醒 — 在合适时机向消息历史注入引导
pub struct NudgeSystem;

impl NudgeSystem {
    /// 连续只读提醒
    pub fn check_consecutive_reads(
        consecutive_reads: usize,
        max_reads: usize,
    ) -> Option<String> {
        if consecutive_reads >= max_reads {
            Some(format!(
                "你已经连续 {} 轮只进行读取操作。是否需要开始写代码或执行修改？",
                consecutive_reads
            ))
        } else {
            None
        }
    }

    /// 接近迭代上限提醒
    pub fn check_iteration_warning(
        iteration: usize,
        max_iterations: usize,
    ) -> Option<String> {
        let remaining = max_iterations.saturating_sub(iteration);
        if remaining <= 3 && remaining > 0 {
            Some(format!(
                "只剩 {} 轮迭代机会了。请尽快给出最终结论，不要再进行探索性操作。",
                remaining
            ))
        } else {
            None
        }
    }

    /// 完成度检查：如果 agent 说"完成了"但没有执行过写入操作，提醒
    pub fn check_completion_without_write(
        has_write_action: bool,
        already_nudged: bool,
    ) -> Option<String> {
        if !has_write_action && !already_nudged {
            Some("你声明了任务完成，但尚未执行任何写入操作。如果有待办事项未完成，请先完成。".into())
        } else {
            None
        }
    }

    /// 注入 nudge 到消息历史
    pub fn inject_nudge(messages: &mut Vec<Message>, nudge: &str) {
        messages.push(Message::user(format!("[系统提醒] {}", nudge)));
    }
}
