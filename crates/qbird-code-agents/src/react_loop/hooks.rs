use qbird_code_models::{Message, ToolCall};

use crate::doom_loop::{DoomLoopAction, DoomLoopDetector};
use crate::nudge::NudgeSystem;

use super::types::{AgentHook, HookAction, LoopState, READ_ONLY_TOOLS};

/// 内置 hook：死循环检测 + Nudge
pub(super) struct AgentHooks {
    doom_detector: DoomLoopDetector,
    config: super::ReactLoopConfig,
    /// 暂存当前迭代的 tool_calls，供 on_llm_response 使用
    current_tool_calls: Vec<ToolCall>,
}

impl AgentHooks {
    pub(super) fn new(config: &super::ReactLoopConfig) -> Self {
        Self {
            doom_detector: DoomLoopDetector::new(),
            config: config.clone(),
            current_tool_calls: Vec::new(),
        }
    }

    /// 记录当前迭代的 tool_calls（在 process_llm_response 中调用）
    pub(super) fn set_tool_calls(&mut self, tool_calls: Vec<ToolCall>) {
        self.current_tool_calls = tool_calls;
    }

    /// 把本轮 tool 执行结果喂给 doom_detector；如果连续失败到上限，
    /// 返回 ForceStop + 警告，调用方应 halt loop。
    pub(super) fn record_tool_outcomes(
        &mut self,
        outcomes: &[bool],
    ) -> (crate::doom_loop::DoomLoopAction, String) {
        self.doom_detector.record_outcomes(outcomes)
    }

    /// 执行 completion nudge 检查（在 process_llm_response 中调用）
    pub(super) fn check_completion_nudge(
        &mut self,
        state: &mut LoopState,
        messages: &[Message],
    ) -> Option<String> {
        if state.completion_nudge_sent {
            return None;
        }
        let has_writes = messages.iter().any(|m| {
            m.tool_calls
                .as_ref()
                .map(|tc| {
                    tc.iter()
                        .any(|c| !READ_ONLY_TOOLS.contains(&c.function.name.as_str()))
                })
                .unwrap_or(false)
        });
        let nudge =
            NudgeSystem::check_completion_without_write(has_writes, state.completion_nudge_sent);
        if nudge.is_some() {
            state.completion_nudge_sent = true;
        }
        nudge
    }

    /// 连续只读 + 无工具调用 + 迭代上限 nudge（LLM 调用前）
    pub(super) fn apply_pre_llm_nudges(
        &mut self,
        state: &mut LoopState,
        messages: &mut Vec<Message>,
    ) {
        if let Some(n) = NudgeSystem::check_consecutive_reads(
            state.consecutive_reads,
            self.config.max_consecutive_reads,
            state.read_nudge_sent,
        ) {
            state.read_nudge_sent = true;
            NudgeSystem::inject_nudge(messages, &n);
        }

        if let Some(n) = NudgeSystem::check_no_tool_calls(
            state.consecutive_no_tool_calls,
            state.no_tool_nudge_sent,
        ) {
            state.no_tool_nudge_sent = true;
            NudgeSystem::inject_nudge(messages, &n);
        }

        if let Some(n) =
            NudgeSystem::check_iteration_warning(state.iteration, self.config.max_iterations)
        {
            NudgeSystem::inject_nudge(messages, &n);
        }
    }
}

impl AgentHook for AgentHooks {
    /// LLM 响应后：死循环检测
    fn on_llm_response(&mut self, _state: &LoopState) -> HookAction {
        if self.current_tool_calls.is_empty() {
            return HookAction::Proceed;
        }

        let (action, warning) = self.doom_detector.check(&self.current_tool_calls);

        match action {
            DoomLoopAction::ForceStop => HookAction::Halt(format!("任务被终止: {}", warning)),
            DoomLoopAction::Redirect | DoomLoopAction::Notify => {
                if let Some(msg) = DoomLoopDetector::recovery_message(&action) {
                    tracing::warn!("Doom loop detected: {}", warning);
                    HookAction::Nudge(msg)
                } else {
                    HookAction::Proceed
                }
            }
            DoomLoopAction::None => HookAction::Proceed,
        }
    }
}
