//! 通用 OpenAI 兼容 adapter
//!
//! v1.3 起替代旧 `OpenAiProvider`。从 ProviderConfig 构造，不读 env var。

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;

use super::check_status;
use super::pick_model;
use super::types::{
    ChatChunk, ChatRequest, ChatResponse, LlmProvider, MessageRole, TokenUsage, ToolCall,
};
use crate::common::error::{EflowError, Result};
use rust_i18n::t;

const CHAT_PATH: &str = "/chat/completions";

pub struct GenericOpenAiProvider {
    id: String,
    api_key: String,
    default_model: String,
    base_url: String,
    client: Client,
    max_retries: u8,
    retry_backoff_ms: u64,
    /// model id → endpoint path（OpenCode Go 场景）
    model_endpoints: HashMap<String, String>,
}

impl GenericOpenAiProvider {
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

    /// 完整 POST URL = base + path（path 可来自 model_endpoints 或默认）
    fn chat_url(&self, model: &str) -> String {
        let path = self
            .model_endpoints
            .get(model)
            .map(String::as_str)
            .unwrap_or(CHAT_PATH);
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn build_post(&self, body: &Value) -> reqwest::RequestBuilder {
        self.client
            .post(self.chat_url(body["model"].as_str().unwrap_or("")))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(body)
    }
}

#[async_trait]
impl LlmProvider for GenericOpenAiProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let model = pick_model(&self.default_model, &request.model);

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

        let response = self.build_post(&body).send().await.map_err(|e| {
            EflowError::LlmProvider(t!("err_http", msg = e.to_string()).to_string())
        })?;
        let response = check_status(response, "OpenAI").await?;

        let json: Value = response.json().await.map_err(|e| {
            EflowError::LlmProvider(t!("err_json_parse", msg = e.to_string()).to_string())
        })?;

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
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_url_uses_default_when_model_endpoints_empty() {
        let p = GenericOpenAiProvider::new(
            "deepseek".into(),
            "k".into(),
            "https://api.deepseek.com".into(),
            "deepseek-v4-pro".into(),
            30,
            3,
            1000,
            HashMap::new(),
        );
        assert_eq!(
            p.chat_url("any-model"),
            "https://api.deepseek.com/chat/completions"
        );
    }

    #[test]
    fn chat_url_uses_model_specific_endpoint() {
        let mut endpoints = HashMap::new();
        endpoints.insert("glm-5.1".to_string(), "/v1/chat/completions".to_string());
        endpoints.insert("kimi-k2.7".to_string(), "/v1/messages".to_string());
        let p = GenericOpenAiProvider::new(
            "opencode-go".into(),
            "k".into(),
            "https://opencode.ai/zen/go/v1".into(),
            "glm-5.1".into(),
            30,
            3,
            1000,
            endpoints,
        );
        assert_eq!(
            p.chat_url("glm-5.1"),
            "https://opencode.ai/zen/go/v1/v1/chat/completions"
        );
        assert_eq!(
            p.chat_url("kimi-k2.7"),
            "https://opencode.ai/zen/go/v1/v1/messages"
        );
    }

    #[test]
    fn name_returns_config_id() {
        let p = GenericOpenAiProvider::new(
            "deepseek".into(),
            "k".into(),
            "https://api.deepseek.com".into(),
            "deepseek-v4-pro".into(),
            30,
            3,
            1000,
            HashMap::new(),
        );
        assert_eq!(p.name(), "deepseek");
    }

    #[test]
    fn retry_params_returns_constructor_values() {
        let p = GenericOpenAiProvider::new(
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
