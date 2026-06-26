pub mod types;

pub use types::{AgentResult, LoopState, ReactLoopConfig, TurnResult};

use std::sync::Arc;

use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::providers::{Provider, RequestConfig};
use qbird_code_models::{EflowError, Message, ToolCall, ToolCallFunction};
use qbird_code_tools::ToolRegistry;

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

    /// 主入口：运行 ReAct 循环
    pub async fn run(
        &self,
        provider: &dyn Provider,
        http_client: &HttpLlmClient,
        messages: &mut Vec<Message>,
        tool_schemas: &[serde_json::Value],
        tool_registry: &Arc<ToolRegistry>,
        max_iterations_override: Option<usize>,
    ) -> Result<AgentResult, EflowError> {
        let max_iters = max_iterations_override.unwrap_or(self.config.max_iterations);
        let mut state = LoopState::new();
        let mut doom_detector = crate::doom_loop::DoomLoopDetector::new();

        loop {
            state.iteration += 1;

            // === 1. 安全自检 ===
            self.check_safety(&state, max_iters, messages)?;

            // === 2. Nudge 检测 ===
            self.check_nudges(&state, messages);

            // === 3. LLM 调用 ===
            let request_config = RequestConfig {
                temperature: self.config.temperature,
                max_tokens: self.config.max_tokens,
                stream: false,
                thinking_enabled: true,
                thinking_effort: Some("high".into()),
                tools: tool_schemas.to_vec(),
            };

            let body = provider.build_request_body(messages, &request_config);
            let response_json = http_client.send(provider, &body).await?;
            let chat_response = provider.parse_response(&response_json).await?;

            // === 4. 响应处理 ===
            // 提取 reasoning_content (DeepSeek)
            let reasoning = chat_response.reasoning_content.clone();

            // 构建 assistant 消息
            let assistant_msg = if let Some(ref tool_calls_json) = chat_response.tool_calls {
                // 有 tool calls: 构建带 tool_calls 的消息
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
                    .all(|tc| types::READ_ONLY_TOOLS.contains(&tc.function.name.as_str()));

                if is_read_only {
                    state.consecutive_reads += 1;
                } else {
                    state.consecutive_reads = 0;
                }

                state.consecutive_no_tool_calls = 0;

                Message::assistant_with_tools(
                    chat_response.content.clone(),
                    reasoning,
                    tool_calls.clone(),
                )
            } else {
                // 无 tool calls
                state.consecutive_no_tool_calls += 1;
                state.consecutive_reads = 0;

                // 检查是否完成
                let finish = chat_response.finish_reason.as_deref();
                if finish == Some("stop") && !chat_response.content.is_empty() {
                    // LLM 宣布完成
                    let has_writes = messages.iter().any(|m| {
                        m.tool_calls
                            .as_ref()
                            .map(|tc| {
                                tc.iter().any(|c| {
                                    !types::READ_ONLY_TOOLS.contains(&c.function.name.as_str())
                                })
                            })
                            .unwrap_or(false)
                    });

                    let nudge = crate::nudge::NudgeSystem::check_completion_without_write(
                        has_writes,
                        state.completion_nudge_sent,
                    );

                    if let Some(n) = nudge {
                        state.completion_nudge_sent = true;
                        crate::nudge::NudgeSystem::inject_nudge(messages, &n);
                        messages.push(Message::assistant(chat_response.content.clone(), reasoning));
                        continue;
                    }

                    messages.push(Message::assistant(chat_response.content.clone(), reasoning));
                    return Ok(AgentResult {
                        content: chat_response.content.clone(),
                        messages: messages.clone(),
                        usage: chat_response.usage.clone(),
                    });
                }

                // 无工具调用但也没完成 → 继续
                Message::assistant(chat_response.content.clone(), reasoning)
            };

            messages.push(assistant_msg);

            // === 5. 死循环检测（仅当有 tool calls 时） ===
            if let Some(ref tc) = chat_response.tool_calls {
                let tool_calls: Vec<ToolCall> = tc
                    .iter()
                    .map(|t| ToolCall {
                        id: t["id"].as_str().unwrap_or("").to_string(),
                        function: ToolCallFunction {
                            name: t["function"]["name"].as_str().unwrap_or("").to_string(),
                            arguments: t["function"]["arguments"]
                                .as_str()
                                .unwrap_or("{}")
                                .to_string(),
                        },
                    })
                    .collect();

                let (action, warning) = doom_detector.check(&tool_calls);

                match action {
                    crate::doom_loop::DoomLoopAction::ForceStop => {
                        return Ok(AgentResult {
                            content: format!("任务被终止: {}", warning),
                            messages: messages.clone(),
                            usage: chat_response.usage.clone(),
                        });
                    }
                    crate::doom_loop::DoomLoopAction::Redirect
                    | crate::doom_loop::DoomLoopAction::Notify => {
                        if let Some(msg) =
                            crate::doom_loop::DoomLoopDetector::recovery_message(&action)
                        {
                            tracing::warn!("Doom loop detected: {}", warning);
                            crate::nudge::NudgeSystem::inject_nudge(messages, &msg);
                        }
                    }
                    crate::doom_loop::DoomLoopAction::None => {}
                }

                // === 6. 工具执行 ===
                let is_all_readonly = tool_calls
                    .iter()
                    .all(|tc| types::READ_ONLY_TOOLS.contains(&tc.function.name.as_str()));

                if is_all_readonly {
                    // 批量并行
                    self.execute_tools_parallel(&tool_calls, tool_registry, messages)
                        .await?;
                } else {
                    // 逐一串行
                    self.execute_tools_sequential(&tool_calls, tool_registry, messages)
                        .await?;
                }
            }
        }
    }

    /// === 安全自检 ===
    fn check_safety(
        &self,
        state: &LoopState,
        max_iters: usize,
        messages: &mut Vec<Message>,
    ) -> Result<(), EflowError> {
        if state.iteration > max_iters {
            // Wind-down: 注入总结请求
            crate::nudge::NudgeSystem::inject_nudge(
                messages,
                "达到最大迭代次数。请立即总结当前状态并给出最终回答。",
            );
            return Err(EflowError::Internal("Max iterations reached".into()));
        }
        Ok(())
    }

    /// === Nudge 检测 ===
    fn check_nudges(&self, state: &LoopState, messages: &mut Vec<Message>) {
        if let Some(n) = crate::nudge::NudgeSystem::check_consecutive_reads(
            state.consecutive_reads,
            self.config.max_consecutive_reads,
        ) {
            crate::nudge::NudgeSystem::inject_nudge(messages, &n);
        }

        if let Some(n) = crate::nudge::NudgeSystem::check_iteration_warning(
            state.iteration,
            self.config.max_iterations,
        ) {
            crate::nudge::NudgeSystem::inject_nudge(messages, &n);
        }
    }

    /// === 工具并行执行（只读） ===
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
            messages.push(Message::tool_result(call_id, call_name, content));
        }

        Ok(())
    }

    /// === 工具串行执行（写入） ===
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

            let result = tool_registry
                .execute(&tc.function.name, args, task_id)
                .await;
            let content = match result {
                Ok(output) => output.content,
                Err(e) => format!("错误: {}", e),
            };
            messages.push(Message::tool_result(
                tc.id.clone(),
                tc.function.name.clone(),
                content,
            ));
        }

        Ok(())
    }
}
