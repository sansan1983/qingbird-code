use qbird_code_models::{EflowError, Message, ToolCall, ToolCallFunction};

use super::hooks::AgentHooks;
use super::types::{AgentResult, LoopState, READ_ONLY_TOOLS, Step};
use crate::nudge::NudgeSystem;

/// 检查是否超过最大迭代次数
pub(super) fn check_max_iterations(state: &LoopState, max_iters: usize) -> Option<String> {
    if state.iteration > max_iters {
        Some(rust_i18n::t!("nudge_max_iterations").to_string())
    } else {
        None
    }
}

/// 处理 LLM 响应：解析 tool_calls，更新 state，构建 assistant 消息
///
/// 返回:
/// - `Step::CallTools` — 有工具需要执行
/// - `Step::Done` — LLM 已完成任务
/// - `Step::CallLlm` — 既无工具也无完成，继续下一轮
pub(super) fn process_llm_response(
    state: &mut LoopState,
    hooks: &mut AgentHooks,
    response: &qbird_code_infra::providers::ChatResponse,
    messages: &mut Vec<Message>,
) -> Result<Step, EflowError> {
    let reasoning = response.reasoning_content.clone();

    // ===== 有 tool_calls 的情况 =====
    if let Some(ref tool_calls_json) = response.tool_calls {
        let tool_calls: Vec<ToolCall> = tool_calls_json
            .iter()
            .map(|tc| {
                let fc = &tc["function"];
                ToolCall {
                    id: tc["id"].as_str().unwrap_or("").to_string(),
                    function: ToolCallFunction {
                        name: fc["name"].as_str().unwrap_or("").to_string(),
                        arguments: fc["arguments"].as_str().unwrap_or("{}").to_string(),
                    },
                }
            })
            .collect();

        let is_read_only = tool_calls
            .iter()
            .all(|tc| READ_ONLY_TOOLS.contains(&tc.function.name.as_str()));

        if is_read_only {
            state.consecutive_reads += 1;
        } else {
            state.consecutive_reads = 0;
        }
        state.consecutive_no_tool_calls = 0;

        // 存 tool_calls 给 hooks 用
        hooks.set_tool_calls(tool_calls.clone());

        let msg =
            Message::assistant_with_tools(response.content.clone(), reasoning, tool_calls.clone());
        messages.push(msg);

        return Ok(Step::CallTools { tool_calls });
    }

    // ===== 无 tool_calls 的情况 =====
    state.consecutive_no_tool_calls += 1;
    state.consecutive_reads = 0;

    let finish = response.finish_reason.as_deref();

    if finish == Some("stop") && !response.content.is_empty() {
        // LLM 宣布完成 → 检查 completion nudge
        if let Some(nudge) = hooks.check_completion_nudge(state, messages) {
            NudgeSystem::inject_nudge(messages, &nudge);
            messages.push(Message::assistant(response.content.clone(), reasoning));
            return Ok(Step::CallLlm);
        }

        // 真的完成了
        messages.push(Message::assistant(response.content.clone(), reasoning));
        return Ok(Step::Done(AgentResult {
            content: response.content.clone(),
            messages: messages.clone(),
            usage: response.usage.clone(),
        }));
    }

    // 无工具调用也没完成 → 继续
    messages.push(Message::assistant(response.content.clone(), reasoning));
    Ok(Step::CallLlm)
}

/// 工具执行后：检查是否需要继续
pub(super) fn after_tool_execution(_state: &LoopState, _hooks: &mut AgentHooks) -> Step {
    Step::CallLlm
}
