use qbird_code_models::Message;
use rust_i18n::t;

/// Nudge 提醒 — 在合适时机向消息历史注入引导
pub struct NudgeSystem;

impl NudgeSystem {
    /// 连续只读提醒
    pub fn check_consecutive_reads(
        consecutive_reads: usize,
        max_reads: usize,
        already_nudged: bool,
    ) -> Option<String> {
        if consecutive_reads >= max_reads && !already_nudged {
            Some(t!("nudge_consecutive_reads", count = consecutive_reads as i64).to_string())
        } else {
            None
        }
    }

    /// 接近迭代上限提醒
    pub fn check_iteration_warning(iteration: usize, max_iterations: usize) -> Option<String> {
        let remaining = max_iterations.saturating_sub(iteration);
        if remaining <= 3 && remaining > 0 {
            Some(t!("nudge_iteration_warning", remaining = remaining as i64).to_string())
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
            Some(t!("nudge_completion_without_write").to_string())
        } else {
            None
        }
    }

    /// 连续无工具调用提醒
    pub fn check_no_tool_calls(
        consecutive_no_tool_calls: usize,
        already_nudged: bool,
    ) -> Option<String> {
        if consecutive_no_tool_calls >= 3 && !already_nudged {
            Some(t!("nudge_no_tool_calls").to_string())
        } else {
            None
        }
    }

    /// 注入 nudge 到消息历史
    pub fn inject_nudge(messages: &mut Vec<Message>, nudge: &str) {
        let prefix = t!("nudge_prefix").to_string();
        messages.push(Message::user(format!("{} {}", prefix, nudge)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_consecutive_reads_below_threshold() {
        let result = NudgeSystem::check_consecutive_reads(3, 5, false);
        assert!(result.is_none());
    }

    #[test]
    fn check_consecutive_reads_at_threshold() {
        let result = NudgeSystem::check_consecutive_reads(5, 5, false);
        assert!(result.is_some());
        let msg = result.unwrap();
        assert!(!msg.is_empty());
    }

    #[test]
    fn check_consecutive_reads_above_threshold() {
        let result = NudgeSystem::check_consecutive_reads(7, 5, false);
        assert!(result.is_some());
    }

    #[test]
    fn check_consecutive_reads_already_nudged() {
        let result = NudgeSystem::check_consecutive_reads(5, 5, true);
        assert!(result.is_none());

        let result = NudgeSystem::check_consecutive_reads(7, 5, true);
        assert!(result.is_none());
    }

    #[test]
    fn check_iteration_warning_far_from_limit() {
        let result = NudgeSystem::check_iteration_warning(10, 50);
        assert!(result.is_none());
    }

    #[test]
    fn check_iteration_warning_close_to_limit() {
        let result = NudgeSystem::check_iteration_warning(47, 50);
        assert!(result.is_some());
    }

    #[test]
    fn check_iteration_warning_at_zero_remaining() {
        let result = NudgeSystem::check_iteration_warning(50, 50);
        assert!(result.is_none());
    }

    #[test]
    fn check_completion_without_write_no_writes_not_nudged() {
        let result = NudgeSystem::check_completion_without_write(false, false);
        assert!(result.is_some());
    }

    #[test]
    fn check_completion_without_write_with_writes() {
        let result = NudgeSystem::check_completion_without_write(true, false);
        assert!(result.is_none());
    }

    #[test]
    fn check_completion_without_write_already_nudged() {
        let result = NudgeSystem::check_completion_without_write(false, true);
        assert!(result.is_none());
    }

    #[test]
    fn check_no_tool_calls_below_threshold() {
        let result = NudgeSystem::check_no_tool_calls(1, false);
        assert!(result.is_none());
    }

    #[test]
    fn check_no_tool_calls_at_threshold() {
        let result = NudgeSystem::check_no_tool_calls(3, false);
        assert!(result.is_some());
    }

    #[test]
    fn check_no_tool_calls_already_nudged() {
        let result = NudgeSystem::check_no_tool_calls(4, true);
        assert!(result.is_none());
    }

    #[test]
    fn inject_nudge_adds_message() {
        let mut messages = vec![];
        NudgeSystem::inject_nudge(&mut messages, "test nudge");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role_str(), "user");
        assert!(messages[0].content.contains("test nudge"));
    }
}
