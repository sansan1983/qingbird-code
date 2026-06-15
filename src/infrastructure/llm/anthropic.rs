use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;

use super::types::{ChatChunk, ChatRequest, ChatResponse, LlmProvider, MessageRole, TokenUsage};
use crate::common::error::{EflowError, Result};
use rust_i18n::t;

pub struct AnthropicProvider {
    api_key: String,
    default_model: String,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(api_key: String, default_model: String) -> Self {
        Self {
            api_key,
            default_model,
            client: Client::new(),
        }
    }

    fn build_body(&self, request: &ChatRequest) -> Value {
        let system_msg = request
            .messages
            .iter()
            .find(|m| m.role == MessageRole::System)
            .map(|m| m.content.clone());

        let messages: Vec<Value> = request
            .messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::System => "system",
                };
                serde_json::json!({ "role": role, "content": m.content })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "max_tokens": request.max_tokens,
            "messages": messages,
        });

        if let Some(system) = system_msg {
            body["system"] = serde_json::json!(system);
        }
        if let Some(ref tools) = request.tools {
            body["tools"] = serde_json::json!(tools);
        }

        body
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };

        let body = self.build_body(&ChatRequest {
            model: model.clone(),
            ..request
        });

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| EflowError::LlmProvider(t!("err_http", msg = e.to_string())))?;

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(EflowError::LlmAuthFailed("Anthropic".into()));
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(EflowError::RateLimited("Anthropic".into()));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| EflowError::LlmProvider(t!("err_json_parse", msg = e.to_string())))?;

        let content = json["content"]
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b["type"] == "text")
                    .map(|b| b["text"].as_str().unwrap_or(""))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();

        let input_tokens = json["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = json["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;

        Ok(ChatResponse {
            content,
            tool_calls: None,
            usage: TokenUsage {
                input_tokens,
                output_tokens,
            },
            finish_reason: json["stop_reason"].as_str().unwrap_or("unknown").into(),
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        let api_key = self.api_key.clone();
        let default_model = self.default_model.clone();

        let model = if request.model.is_empty() {
            default_model
        } else {
            request.model
        };

        let mut body = self.build_body(&ChatRequest {
            model: model.clone(),
            ..request
        });
        body["stream"] = serde_json::json!(true);

        let (tx, rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let client = Client::new();
            let response = match client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx
                        .send(Err(EflowError::LlmProvider(format!("{}", e))))
                        .await;
                    return;
                }
            };

            use futures_util::StreamExt;
            let mut stream = response.bytes_stream();
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        for line in text.lines() {
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    continue;
                                }
                                if let Ok(json) = serde_json::from_str::<Value>(data) {
                                    let content_delta =
                                        json["delta"]["text"].as_str().map(|s| s.to_string());
                                    let _ = tx
                                        .send(Ok(ChatChunk {
                                            content_delta,
                                            tool_call_delta: None,
                                        }))
                                        .await;
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(rx)
    }

    fn supports_prefix_cache(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "anthropic"
    }
}
