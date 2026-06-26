use std::collections::VecDeque;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use qbird_code_models::ToolCall;

const MAX_CYCLE_LEN: usize = 3;
const DOOM_LOOP_THRESHOLD: usize = 3;
const MAX_RECENT: usize = 20;

/// 死循环升级级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoomLoopAction {
    None,
    Redirect,
    Notify,
    ForceStop,
}

/// 恢复策略
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    Nudge(String),
    StepBack(String),
    CompactContext,
}

/// 死循环检测器
#[derive(Debug)]
pub struct DoomLoopDetector {
    recent: VecDeque<String>,
    nudge_count: usize,
}

impl DoomLoopDetector {
    pub fn new() -> Self {
        Self {
            recent: VecDeque::with_capacity(MAX_RECENT),
            nudge_count: 0,
        }
    }

    pub fn nudge_count(&self) -> usize { self.nudge_count }

    pub fn reset(&mut self) {
        self.recent.clear();
        self.nudge_count = 0;
    }

    /// 计算工具调用指纹: "tool_name:args_hash"
    fn fingerprint(tool_name: &str, args_str: &str) -> String {
        let mut hasher = DefaultHasher::new();
        args_str.hash(&mut hasher);
        let h = hasher.finish();
        format!("{}:{:016x}", tool_name, h)
    }

    /// 检测是否进入死循环。返回 (action, warning_message)
    pub fn check(&mut self, tool_calls: &[ToolCall]) -> (DoomLoopAction, String) {
        // 追加指纹
        for tc in tool_calls {
            let args = &tc.function.arguments;
            let fp = Self::fingerprint(&tc.function.name, args);
            if self.recent.len() >= MAX_RECENT {
                self.recent.pop_front();
            }
            self.recent.push_back(fp);
        }

        let tail: Vec<&String> = self.recent.iter().collect();

        for cycle_len in 1..=MAX_CYCLE_LEN {
            let required = cycle_len * DOOM_LOOP_THRESHOLD;
            if tail.len() < required {
                continue;
            }

            let segment = &tail[tail.len() - required..];
            let pattern = &segment[..cycle_len];
            let is_cycle = segment.iter().enumerate()
                .all(|(i, fp)| *fp == pattern[i % cycle_len]);

            if is_cycle {
                self.nudge_count += 1;

                // 提取信息用于构建 warning（在修改 recent 之前）
                let warning = if cycle_len == 1 {
                    let tool_name = pattern[0].split(':').next().unwrap_or("unknown");
                    format!(
                        "Agent 已连续使用 `{}` {} 次，参数完全相同，可能陷入死循环。",
                        tool_name, DOOM_LOOP_THRESHOLD
                    )
                } else {
                    let names: Vec<&str> = pattern.iter()
                        .map(|p| p.split(':').next().unwrap_or("?"))
                        .collect();
                    format!(
                        "Agent 重复 {}-步循环 ({}) {} 次，可能陷入死循环。",
                        cycle_len, names.join(" → "), DOOM_LOOP_THRESHOLD
                    )
                };

                // 移除已检测到的循环段，防止立即重复触发
                for _ in 0..required {
                    self.recent.pop_back();
                }

                let action = match self.nudge_count {
                    1 => DoomLoopAction::Redirect,
                    2 => DoomLoopAction::Notify,
                    _ => DoomLoopAction::ForceStop,
                };

                return (action, warning);
            }
        }

        (DoomLoopAction::None, String::new())
    }

    /// 根据检测结果生成恢复消息
    pub fn recovery_message(action: &DoomLoopAction) -> Option<String> {
        match action {
            DoomLoopAction::Redirect => Some(
                "你似乎陷入了重复的循环。请换一种方式来解决问题。".into()
            ),
            DoomLoopAction::Notify => Some(
                "你已多次重复相同的操作。建议从根本上重新思考你的方法，而不是继续当前路径。".into()
            ),
            DoomLoopAction::ForceStop => Some(
                "由于反复执行相同的操作，此任务已被强制终止。请总结当前的发现，然后宣布完成。".into()
            ),
            DoomLoopAction::None => None,
        }
    }
}

impl Default for DoomLoopDetector {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool_call(name: &str, args: &str) -> ToolCall {
        ToolCall {
            id: "call_1".into(),
            function: qbird_code_models::ToolCallFunction {
                name: name.into(),
                arguments: args.into(),
            },
        }
    }

    #[test]
    fn single_repeat_3_times_is_doom_loop() {
        let mut detector = DoomLoopDetector::new();
        let tc = make_tool_call("read_file", r#"{"path":"a.txt"}"#);

        let (a1, _) = detector.check(&[tc.clone()]); // 1st
        assert_eq!(a1, DoomLoopAction::None);
        let (a2, _) = detector.check(&[tc.clone()]); // 2nd
        assert_eq!(a2, DoomLoopAction::None);
        let (a3, w) = detector.check(&[tc.clone()]); // 3rd → doom!
        assert_eq!(a3, DoomLoopAction::Redirect);
        assert!(w.contains("read_file"));
    }

    #[test]
    fn different_args_no_doom_loop() {
        let mut detector = DoomLoopDetector::new();
        let tc1 = make_tool_call("read_file", r#"{"path":"a.txt"}"#);
        let tc2 = make_tool_call("read_file", r#"{"path":"b.txt"}"#);
        let tc3 = make_tool_call("read_file", r#"{"path":"c.txt"}"#);

        detector.check(&[tc1]);
        detector.check(&[tc2]);
        let (a, _) = detector.check(&[tc3]);
        assert_eq!(a, DoomLoopAction::None); // 不同参数，不触发
    }

    #[test]
    fn second_detection_escalates() {
        let mut detector = DoomLoopDetector::new();
        let tc = make_tool_call("grep", r#"{"pattern":"foo"}"#);

        // Round 1: 3 repeats → Redirect
        for _ in 0..3 { detector.check(&[tc.clone()]); }
        assert_eq!(detector.nudge_count(), 1);

        // Round 2: 3 more repeats → Notify
        for _ in 0..3 { detector.check(&[tc.clone()]); }
        assert_eq!(detector.nudge_count(), 2);

        // Round 3: 3 more repeats → ForceStop
        for _ in 0..3 { detector.check(&[tc.clone()]); }
        assert_eq!(detector.nudge_count(), 3);
    }
}
