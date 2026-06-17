//! 通用 Anthropic 兼容 adapter
//!
//! v1.3 起替代旧 `AnthropicProvider`。从 ProviderConfig 构造，不读 env var。

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;

use super::check_status;
use super::pick_model;
use super::types::{ChatChunk, ChatRequest, ChatResponse, LlmProvider, MessageRole, TokenUsage};
use crate::common::error::{EflowError, Result};
use rust_i18n::t;

const MESSAGES_PATH: &str = "/v1/messages";

pub struct GenericAnthropicProvider {
    id: String,
    api_key: String,
    default_model: String,
    base_url: String,
    client: Client,
    max_retries: u8,
    retry_backoff_ms: u64,
    model_endpoints: HashMap<String, String>,
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
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("reqwest client build");
        Self {
            id,
            api_key,
            default_model,
            base_url,
            client,
            max_retries,
            retry_backoff_ms,
            model_endpoints,
        }
    }

    pub fn retry_params(&self) -> (u8, u64) {
        (self.max_retries, self.retry_backoff_ms)
    }

    fn messages_url(&self, model: &str) -> String {
        let path = self
            .model_endpoints
            .get(model)
            .map(String::as_str)
            .unwrap_or(MESSAGES_PATH);
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn build_post(&self, body: &Value) -> reqwest::RequestBuilder {
        self.client
            .post(self.messages_url(body["model"].as_str().unwrap_or("")))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(body)
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
        let model = pick_model(&self.default_model, &request.model);
        let body = self.build_body(&ChatRequest {
            model: model.clone(),
            ..request
        });

        let response = self.build_post(&body).send().await.map_err(|e| {
            EflowError::LlmProvider(t!("err_http", msg = e.to_string()).to_string())
        })?;
        let response = check_status(response, "Anthropic").await?;

        let json: Value = response.json().await.map_err(|e| {
            EflowError::LlmProvider(t!("err_json_parse", msg = e.to_string()).to_string())
        })?;

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
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_url_uses_default_when_model_endpoints_empty() {
        let p = GenericAnthropicProvider::new(
            "anthropic".into(),
            "k".into(),
            "https://api.anthropic.com".into(),
            "claude-sonnet-4-6".into(),
            30,
            3,
            1000,
            HashMap::new(),
        );
        assert_eq!(
            p.messages_url("any"),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn messages_url_trims_trailing_slash() {
        let p = GenericAnthropicProvider::new(
            "minimax".into(),
            "k".into(),
            "https://api.minimaxi.com/anthropic/".into(),
            "MiniMax-M3".into(),
            30,
            3,
            1000,
            HashMap::new(),
        );
        assert_eq!(
            p.messages_url("any"),
            "https://api.minimaxi.com/anthropic/v1/messages"
        );
    }

    #[test]
    fn messages_url_uses_model_specific_endpoint() {
        let mut endpoints = HashMap::new();
        endpoints.insert("qwen3.7-max".to_string(), "/v1/messages".to_string());
        let p = GenericAnthropicProvider::new(
            "opencode-go".into(),
            "k".into(),
            "https://opencode.ai/zen/go/v1".into(),
            "qwen3.7-max".into(),
            30,
            3,
            1000,
            endpoints,
        );
        assert_eq!(
            p.messages_url("qwen3.7-max"),
            "https://opencode.ai/zen/go/v1/v1/messages"
        );
    }

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
        );
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
        );
        assert_eq!(p.retry_params(), (5, 2000));
    }
}
