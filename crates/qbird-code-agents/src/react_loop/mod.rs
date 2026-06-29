pub mod hooks;
pub mod r#loop;
pub mod types;

pub use types::{AgentHook, AgentResult, HookAction, LoopState, ReactLoopConfig, Step};

use std::sync::Arc;

use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::memory::{ContextManager, MemoryManager};
use qbird_code_infra::providers::{Provider, RequestConfig, StreamEvent};
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
        memory_manager: Option<Arc<MemoryManager>>,
    ) -> Result<AgentResult, EflowError> {
        let max_iters = max_iterations_override.unwrap_or(self.config.max_iterations);
        let mut state = LoopState::new();
        let mut hooks = AgentHooks::new(&self.config);
        let mut memory_injected_this_turn = false;

        loop {
            state.iteration += 1;

            // === 19-02: MemoryManager recall — inject once per user turn ===
            if let Some(ref mm) = memory_manager
                && !memory_injected_this_turn
            {
                if let Some(last_user) = messages.iter().rev().find(|m| m.role_str() == "user") {
                    let recalled = mm.recall(&last_user.content, 500).await;
                    if !recalled.is_empty() {
                        let mut prefix = String::from("[memory]\n");
                        for r in &recalled {
                            prefix.push_str(&r.entry.body);
                            prefix.push('\n');
                        }
                        messages.push(Message::system(prefix));
                        tracing::info!("Memory recall: {} entries injected", recalled.len());
                    }
                }
                memory_injected_this_turn = true;
            }

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

            // === IO: 调 LLM（流式或非流式） ===
            let request_config = RequestConfig {
                model: self.config.model.clone(),
                temperature: self.config.temperature,
                max_tokens: self.config.max_tokens,
                stream: self.config.stream_enabled,
                thinking_enabled: self.config.thinking_enabled,
                thinking_effort: if self.config.thinking_effort.is_empty() {
                    None
                } else {
                    Some(self.config.thinking_effort.clone())
                },
                tools: tool_schemas.to_vec(),
            };

            let chat_response = if self.config.stream_enabled {
                // Streaming path: try stream, fallback to non-streaming
                match provider
                    .stream(http_client, messages, &request_config)
                    .await
                {
                    Ok(mut rx) => {
                        let mut accumulated_content = String::new();
                        let mut accumulated_reasoning = String::new();
                        let mut tool_call_buffers: std::collections::HashMap<
                            usize,
                            (Option<String>, Option<String>, String),
                        > = std::collections::HashMap::new();
                        let mut final_resp = None;

                        while let Some(event) = rx.recv().await {
                            match event {
                                StreamEvent::TextDelta(text) => {
                                    print!("{}", text);
                                    use std::io::Write;
                                    let _ = std::io::stdout().flush();
                                    accumulated_content.push_str(&text);
                                }
                                StreamEvent::ReasoningDelta(text) => {
                                    accumulated_reasoning.push_str(&text);
                                }
                                StreamEvent::ToolCallDelta { index, delta } => {
                                    let entry = tool_call_buffers.entry(index).or_default();
                                    if let Some(id) = delta.id {
                                        entry.0 = Some(id);
                                    }
                                    if let Some(name) = delta.name {
                                        entry.1 = Some(name);
                                    }
                                    entry.2.push_str(&delta.arguments_delta);
                                }
                                StreamEvent::Done(mut resp) => {
                                    // Fill accumulated content into response
                                    resp.content = accumulated_content.clone();
                                    if !accumulated_reasoning.is_empty() {
                                        resp.reasoning_content =
                                            Some(accumulated_reasoning.clone());
                                    }
                                    // Build tool_calls from buffers
                                    if !tool_call_buffers.is_empty() {
                                        let mut sorted: Vec<_> = tool_call_buffers.iter().collect();
                                        sorted.sort_by_key(|(i, _)| **i);
                                        let calls: Vec<serde_json::Value> = sorted
                                            .iter()
                                            .map(|(_, (id, name, args))| {
                                                serde_json::json!({
                                                    "id": id.clone().unwrap_or_default(),
                                                    "type": "function",
                                                    "function": {
                                                        "name": name.clone().unwrap_or_default(),
                                                        "arguments": args,
                                                    }
                                                })
                                            })
                                            .collect();
                                        resp.tool_calls = Some(calls);
                                    }
                                    final_resp = Some(resp);
                                    break;
                                }
                                StreamEvent::Error(e) => {
                                    tracing::warn!(
                                        "Stream error, falling back to non-streaming: {}",
                                        e
                                    );
                                    break;
                                }
                            }
                        }
                        println!(); // newline after streaming

                        match final_resp {
                            Some(r) => r,
                            None => {
                                // Fallback to non-streaming
                                tracing::warn!(
                                    "Streaming incomplete, falling back to non-streaming"
                                );
                                let body = provider.build_request_body(messages, &request_config);
                                let response_json = http_client.send(provider, &body).await?;
                                provider.parse_response(&response_json).await?
                            }
                        }
                    }
                    Err(e) => {
                        // Stream init failed, fallback to non-streaming
                        tracing::warn!("Stream init failed ({}), falling back to non-streaming", e);
                        let body = provider.build_request_body(messages, &request_config);
                        let response_json = http_client.send(provider, &body).await?;
                        provider.parse_response(&response_json).await?
                    }
                }
            } else {
                // Non-streaming path (existing)
                let body = provider.build_request_body(messages, &request_config);
                let response_json = http_client.send(provider, &body).await?;
                provider.parse_response(&response_json).await?
            };

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

            // 19-01: keep ContextManager in sync with the live message log,
            // and check for a checkpoint.  The assistant message pushed by
            // `process_llm_response` is always the most recent entry.
            if let Some(ref mut cm) = context_manager {
                if let Some(last) = messages.last() {
                    cm.add_chat_message(last);
                }
                if let Some(event) = cm.checkpoint_if_needed() {
                    tracing::info!("Context checkpoint: {:?}", event);
                }
            }

            // 19-02: save assistant content to memory (async, fire-and-forget)
            if let Some(ref mm) = memory_manager
                && let Some(assistant_msg) =
                    messages.iter().rev().find(|m| m.role_str() == "assistant")
            {
                let content = assistant_msg.content.clone();
                let path = format!("turn-{}", state.iteration);
                if let Ok(handle) =
                    mm.clone()
                        .save_with_summarization(content, "user".into(), Some(&path))
                {
                    tokio::spawn(async move {
                        if let Err(e) = handle.await {
                            tracing::warn!("Memory save failed: {}", e);
                        }
                    });
                }
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
                        self.execute_tools_parallel(
                            &tool_calls,
                            tool_registry,
                            messages,
                            provider,
                            http_client,
                        )
                        .await?;
                    } else {
                        self.execute_tools_sequential(
                            &tool_calls,
                            tool_registry,
                            messages,
                            provider,
                            http_client,
                        )
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
        provider: &dyn Provider,
        http_client: &HttpLlmClient,
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
            let is_delegate = call_name == "delegate_task";
            let tc_clone = tc.clone();

            println!(
                "⚙ {}({})",
                call_name,
                truncate_str(&tc.function.arguments, 120)
            );
            futures.push(async move {
                let result = if is_delegate {
                    self.execute_delegate_task(&tc_clone, provider, http_client)
                        .await
                } else {
                    registry
                        .execute(&name, args, task_id)
                        .await
                        .map(|o| o.content)
                };
                (call_id, call_name, result)
            });
        }

        while let Some((call_id, call_name, result)) = futures.next().await {
            let content = match result {
                Ok(c) => c,
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
        provider: &dyn Provider,
        http_client: &HttpLlmClient,
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
            let content = if tc.function.name == "delegate_task" {
                self.execute_delegate_task(tc, provider, http_client)
                    .await?
            } else {
                let result = tool_registry
                    .execute(&tc.function.name, args, task_id)
                    .await;
                match result {
                    Ok(output) => output.content,
                    Err(e) => format!("错误: {}", e),
                }
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

    /// 跑 `delegate_task` 工具调用：调 `SubagentExecutor::spawn_child_with_provider`
    /// 并把 ChildRecord 序列化成 pretty JSON 返回（与 DelegateTaskTool.execute_with_provider
    /// 输出格式一致，避免 ReactLoop 直接接 SubagentExecutor 时与 DelegateTaskTool 输出分裂）。
    async fn execute_delegate_task(
        &self,
        tc: &ToolCall,
        provider: &dyn Provider,
        http_client: &HttpLlmClient,
    ) -> Result<String, EflowError> {
        let executor = self.config.subagent_executor.as_ref().ok_or_else(|| {
            EflowError::Internal("delegate_task called but subagent_executor is None".into())
        })?;

        let args: serde_json::Value =
            serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::json!({}));

        let prompt = args
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EflowError::Internal("delegate_task: missing prompt".into()))?;
        let label = args
            .get("label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EflowError::Internal("delegate_task: missing label".into()))?;
        let profile = args
            .get("profile")
            .and_then(|v| v.as_str())
            .unwrap_or("general");

        let hints = crate::subagent::SubagentSpawnHints::default();
        let record = executor
            .spawn_child_with_provider(profile, prompt, &hints, provider, http_client)
            .await?;

        let output = serde_json::json!({
            "child_id": record.child_id,
            "label": label,
            "status": format!("{:?}", record.status),
            "summary": record.summary,
            "profile": record.profile,
            "tool_policy": format!("{:?}", record.tool_policy),
            "duration_ms": record.duration_ms,
        });

        Ok(serde_json::to_string_pretty(&output).unwrap_or_default())
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
