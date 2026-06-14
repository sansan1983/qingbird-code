use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;

use super::types::{
    ChatChunk, ChatRequest, ChatResponse, LlmProvider, MessageRole, TokenUsage, ToolCall,
};
use crate::common::error::{EflowError, Result};

pub struct OpenAiProvider {
    api_key: String,
    default_model: String,
    client: Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String, default_model: String) -> Self {
        Self {
            api_key,
            default_model,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };

        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                };
                serde_json::json!({ "role": role, "content": m.content })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        });

        if let Some(ref tools) = request.tools {
            body["tools"] = serde_json::json!(tools);
        }

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| EflowError::LlmProvider(format!("HTTP error: {}", e)))?;

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(EflowError::LlmAuthFailed("OpenAI".into()));
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(EflowError::RateLimited("OpenAI".into()));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| EflowError::LlmProvider(format!("JSON parse error: {}", e)))?;

        let choice = &json["choices"][0];
        let msg = &choice["message"];
        let content = msg["content"].as_str().unwrap_or("").to_string();

        let tool_calls = msg["tool_calls"].as_array().map(|tc| {
            tc.iter()
                .map(|t| ToolCall {
                    id: t["id"].as_str().unwrap_or("").into(),
                    name: t["function"]["name"].as_str().unwrap_or("").into(),
                    arguments: t["function"]["arguments"].clone(),
                })
                .collect()
        });

        let input_tokens = json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32;

        Ok(ChatResponse {
            content,
            tool_calls,
            usage: TokenUsage {
                input_tokens,
                output_tokens,
            },
            finish_reason: choice["finish_reason"].as_str().unwrap_or("unknown").into(),
        })
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        Err(EflowError::Internal(
            "OpenAI streaming not yet implemented".into(),
        ))
    }

    fn supports_prefix_cache(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "openai"
    }
}
