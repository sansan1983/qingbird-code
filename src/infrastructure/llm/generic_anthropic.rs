//! 通用 Anthropic 兼容 adapter
//!
//! v1.3 起替代旧 `AnthropicProvider`。从 ProviderConfig 构造，不读 env var。

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

use super::http_client::{HttpClientConfig, HttpLlmClient, LlmProtocol};
use super::pick_model;
use super::types::{ChatChunk, ChatRequest, ChatResponse, LlmProvider, MessageRole};
use crate::common::error::{EflowError, Result};
use rust_i18n::t;

const MESSAGES_PATH: &str = "/v1/messages";

pub struct GenericAnthropicProvider {
    client: HttpLlmClient<AnthropicProtocol>,
}

impl GenericAnthropicProvider {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        api_key: String,
        base_url: String,
        default_model: String,
        timeout_secs: u64,
        max_retries: u8,
        retry_backoff_ms: u64,
        model_endpoints: HashMap<String, String>,
    ) -> Result<Self> {
        let config = HttpClientConfig {
            id,
            api_key,
            default_model,
            base_url,
            timeout_secs,
            max_retries,
            retry_backoff_ms,
            model_endpoints,
        };

        let protocol = AnthropicProtocol;
        let client = HttpLlmClient::new(config, protocol)?;

        Ok(Self { client })
    }

    pub fn retry_params(&self) -> (u8, u64) {
        self.client.retry_params()
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
            body["system"] = serde_json::json!([{
                "type": "text",
                "text": system,
                "cache_control": { "type": "ephemeral" }
            }]);
        }
        if let Some(ref tools) = request.tools {
            body["tools"] = serde_json::json!(tools);
        }

        body
    }
}

#[async_trait]
impl LlmProvider for GenericAnthropicProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let model = pick_model(&self.client.config.default_model, &request.model);
        let body = self.build_body(&ChatRequest {
            model: model.clone(),
            ..request
        });

        let response = self.client.build_post(&body).send().await.map_err(|e| {
            EflowError::LlmProvider(t!("err_http", msg = e.to_string()).to_string())
        })?;
        let response = super::http_client::check_status(response, &self.client.config.id).await?;

        let json: Value = response.json().await.map_err(|e| {
            EflowError::LlmProvider(t!("err_json_parse", msg = e.to_string()).to_string())
        })?;

        let (content, tool_calls, usage, finish_reason) =
            self.client.parse_chat_response(&json).await?;

        Ok(ChatResponse {
            content,
            tool_calls,
            usage,
            finish_reason,
        })
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        Err(EflowError::Internal(
            "Anthropic streaming not yet implemented".into(),
        ))
    }

    fn supports_prefix_cache(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        &self.client.config.id
    }
}

struct AnthropicProtocol;

impl LlmProtocol for AnthropicProtocol {
    fn get_default_path(&self) -> &'static str {
        MESSAGES_PATH
    }

    fn build_auth_headers(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .header("x-api-key", "PLACEHOLDER_API_KEY")
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
    }

    fn parse_chat_response(
        &self,
        json: &Value,
    ) -> (
        String,
        Option<Vec<crate::infrastructure::llm::types::ToolCall>>,
        crate::infrastructure::llm::types::TokenUsage,
        String,
    ) {
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

        let tool_calls = None;

        let input_tokens = json["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = json["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;
        let usage = crate::infrastructure::llm::types::TokenUsage {
            input_tokens,
            output_tokens,
        };

        let finish_reason = json["stop_reason"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        (content, tool_calls, usage, finish_reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_returns_config_id() {
        let p = GenericAnthropicProvider::new(
            "minimax".into(),
            "k".into(),
            "https://api.minimaxi.com/anthropic".into(),
            "MiniMax-M3".into(),
            30,
            3,
            1000,
            HashMap::new(),
        )
        .unwrap();
        assert_eq!(p.name(), "minimax");
    }

    #[test]
    fn retry_params_returns_constructor_values() {
        let p = GenericAnthropicProvider::new(
            "x".into(),
            "k".into(),
            "https://api.test.com".into(),
            "m".into(),
            60,
            5,
            2000,
            HashMap::new(),
        )
        .unwrap();
        assert_eq!(p.retry_params(), (5, 2000));
    }
}
