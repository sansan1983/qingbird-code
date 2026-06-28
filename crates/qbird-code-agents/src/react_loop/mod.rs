pub mod hooks;
pub mod r#loop;
pub mod types;

pub use types::{AgentHook, AgentResult, HookAction, LoopState, ReactLoopConfig, Step};

use std::sync::Arc;

use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::memory::ContextManager;
use qbird_code_infra::providers::{Provider, RequestConfig};
use qbird_code_models::{EflowError, Message, ToolCall, UsageStats};
use qbird_code_tools::ToolRegistry;

use crate::nudge::NudgeSystem;
use hooks::AgentHooks;

pub struct ReactLoop {
    pub config: ReactLoopConfig,
}

impl ReactLoop {
    pub fn new(config: ReactLoopConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(ReactLoopConfig::default())
    }

    /// 主入口：运行 ReAct 循环（外部接口不变）
    #[allow(clippy::too_many_arguments)]
    pub async fn run(
        &self,
        provider: &dyn Provider,
        http_client: &HttpLlmClient,
        messages: &mut Vec<Message>,
        tool_schemas: &[serde_json::Value],
        tool_registry: &Arc<ToolRegistry>,
        max_iterations_override: Option<usize>,
        mut context_manager: Option<&mut ContextManager>,
    ) -> Result<AgentResult, EflowError> {
        let max_iters = max_iterations_override.unwrap_or(self.config.max_iterations);
        let mut state = LoopState::new();
        let mut hooks = AgentHooks::new(&self.config);

        loop {
            state.iteration += 1;

            // === 决策: 超限检查 ===
            if let Some(msg) = r#loop::check_max_iterations(&state, max_iters) {
                NudgeSystem::inject_nudge(messages, &msg);
                return Ok(AgentResult {
                    content: msg,
                    messages: messages.clone(),
                    usage: UsageStats {
                        prompt_tokens: state.total_prompt_tokens,
                        completion_tokens: state.total_completion_tokens,
                        ..Default::default()
                    },
                });
            }

            // === Nudge 检测（连续只读/无工具/接近上限） ===
            hooks.apply_pre_llm_nudges(&mut state, messages);

            // === IO: 调 LLM ===
            let request_config = RequestConfig {
                temperature: self.config.temperature,
                max_tokens: self.config.max_tokens,
                stream: false,
                thinking_enabled: self.config.thinking_enabled,
                thinking_effort: if self.config.thinking_effort.is_empty() {
                    None
                } else {
                    Some(self.config.thinking_effort.clone())
                },
                tools: tool_schemas.to_vec(),
            };

            let body = provider.build_request_body(messages, &request_config);
            let response_json = http_client.send(provider, &body).await?;
            let chat_response = provider.parse_response(&response_json).await?;

            state.total_prompt_tokens += chat_response.usage.prompt_tokens;
            state.total_completion_tokens += chat_response.usage.completion_tokens;
            tracing::info!(
                "Token usage: prompt={}, completion={}, total={}",
                chat_response.usage.prompt_tokens,
                chat_response.usage.completion_tokens,
                chat_response.usage.prompt_tokens + chat_response.usage.completion_tokens,
            );

            // === 决策: LLM 说了什么？ ===
            let step =
                r#loop::process_llm_response(&mut state, &mut hooks, &chat_response, messages)?;

            // 更新 ContextManager 检查点
            if let Some(ref mut cm) = context_manager
                && let Some(event) = cm.checkpoint_if_needed()
            {
                tracing::info!("Context checkpoint: {:?}", event);
            }

            match step {
                Step::CallTools { tool_calls } => {
                    // === 安全: 死循环检测 ===
                    let hook_action = hooks.on_llm_response(&state);
                    match hook_action {
                        HookAction::Halt(msg) => {
                            return Ok(AgentResult {
                                content: msg,
                                messages: messages.clone(),
                                usage: UsageStats {
                                    prompt_tokens: state.total_prompt_tokens,
                                    completion_tokens: state.total_completion_tokens,
                                    ..Default::default()
                                },
                            });
                        }
                        HookAction::Nudge(n) => {
                            NudgeSystem::inject_nudge(messages, &n);
                        }
                        HookAction::Proceed => {}
                    }

                    // === IO: 执行工具 ===
                    let is_all_readonly = tool_calls
                        .iter()
                        .all(|tc| types::READ_ONLY_TOOLS.contains(&tc.function.name.as_str()));

                    if is_all_readonly {
                        self.execute_tools_parallel(&tool_calls, tool_registry, messages)
                            .await?;
                    } else {
                        self.execute_tools_sequential(&tool_calls, tool_registry, messages)
                            .await?;
                    }

                    // === 决策: 工具执行后继续 ===
                    let next = r#loop::after_tool_execution(&state, &mut hooks);
                    match next {
                        Step::CallLlm => continue,
                        Step::Done(r) => return Ok(r),
                        _ => continue,
                    }
                }
                Step::Done(result) => return Ok(result),
                Step::CallLlm => continue,
            }
        }
    }

    // ===== 工具执行（内部辅助，带 IO） =====

    async fn execute_tools_parallel(
        &self,
        tool_calls: &[ToolCall],
        tool_registry: &Arc<ToolRegistry>,
        messages: &mut Vec<Message>,
    ) -> Result<(), EflowError> {
        use futures_util::stream::{FuturesUnordered, StreamExt};

        let task_id = uuid::Uuid::new_v4();
        let mut futures = FuturesUnordered::new();

        for tc in tool_calls {
            let registry = Arc::clone(tool_registry);
            let name = tc.function.name.clone();
            let args: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::json!({}));
            let call_id = tc.id.clone();
            let call_name = tc.function.name.clone();

            println!(
                "⚙ {}({})",
                call_name,
                truncate_str(&tc.function.arguments, 120)
            );
            futures.push(async move {
                let result = registry.execute(&name, args, task_id).await;
                (call_id, call_name, result)
            });
        }

        while let Some((call_id, call_name, result)) = futures.next().await {
            let content = match result {
                Ok(output) => output.content,
                Err(e) => format!("错误: {}", e),
            };
            print_tool_output(&call_name, &content);
            messages.push(Message::tool_result(call_id, call_name, content));
        }

        Ok(())
    }

    async fn execute_tools_sequential(
        &self,
        tool_calls: &[ToolCall],
        tool_registry: &Arc<ToolRegistry>,
        messages: &mut Vec<Message>,
    ) -> Result<(), EflowError> {
        let task_id = uuid::Uuid::new_v4();

        for tc in tool_calls {
            let args: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::json!({}));

            println!(
                "⚙ {}({})",
                tc.function.name,
                truncate_str(&tc.function.arguments, 120)
            );
            let result = tool_registry
                .execute(&tc.function.name, args, task_id)
                .await;
            let content = match result {
                Ok(output) => output.content,
                Err(e) => format!("错误: {}", e),
            };
            print_tool_output(&tc.function.name, &content);
            messages.push(Message::tool_result(
                tc.id.clone(),
                tc.function.name.clone(),
                content,
            ));
        }

        Ok(())
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

fn print_tool_output(tool_name: &str, content: &str) {
    let preview = if content.len() > 800 {
        format!(
            "{}…\n(truncated, {} bytes total)",
            &content[..800],
            content.len()
        )
    } else {
        content.to_string()
    };
    println!("[{}]\n{}\n", tool_name, preview);
}
