//! 通用 OpenAI 兼容 adapter
//!
//! v1.3 起替代旧 `OpenAiProvider`。从 ProviderConfig 构造，不读 env var。

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

use super::http_client::{HttpClientConfig, HttpLlmClient, LlmProtocol};
use super::pick_model;
use super::types::{ChatChunk, ChatRequest, ChatResponse, LlmProvider, MessageRole};
use crate::common::error::{EflowError, Result};
use rust_i18n::t;

const CHAT_PATH: &str = "/chat/completions";

pub struct GenericOpenAiProvider {
    client: HttpLlmClient<OpenAiProtocol>,
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

        let protocol = OpenAiProtocol;
        let client = HttpLlmClient::new(config, protocol)?;

        Ok(Self { client })
    }

    pub fn retry_params(&self) -> (u8, u64) {
        self.client.retry_params()
    }
}

#[async_trait]
impl LlmProvider for GenericOpenAiProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let model = pick_model(&self.client.config.default_model, &request.model);

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
            "OpenAI streaming not yet implemented".into(),
        ))
    }

    fn supports_prefix_cache(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        &self.client.config.id
    }
}

struct OpenAiProtocol;

impl LlmProtocol for OpenAiProtocol {
    fn get_default_path(&self) -> &'static str {
        CHAT_PATH
    }

    fn build_auth_headers(
        &self,
        request: reqwest::RequestBuilder,
        api_key: &str,
    ) -> reqwest::RequestBuilder {
        request
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
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
        let choice = match json.get("choices").and_then(|c| c.get(0)) {
            Some(c) => c,
            None => {
                return (
                    String::new(),
                    None,
                    Default::default(),
                    "empty_choices".into(),
                );
            }
        };
        let msg = &choice["message"];
        let content = msg["content"].as_str().unwrap_or("").to_string();

        let tool_calls = msg["tool_calls"].as_array().map(|tc| {
            tc.iter()
                .map(|t| crate::infrastructure::llm::types::ToolCall {
                    id: t["id"].as_str().unwrap_or("").into(),
                    name: t["function"]["name"].as_str().unwrap_or("").into(),
                    arguments: t["function"]["arguments"].clone(),
                })
                .collect()
        });

        let input_tokens = json["usage"]["prompt_tokens"]
            .as_u64()
            .or_else(|| json["usage"]["input_tokens"].as_u64())
            .unwrap_or(0) as u32;
        let output_tokens = json["usage"]["completion_tokens"]
            .as_u64()
            .or_else(|| json["usage"]["output_tokens"].as_u64())
            .unwrap_or(0) as u32;
        let usage = crate::infrastructure::llm::types::TokenUsage {
            input_tokens,
            output_tokens,
        };

        let finish_reason = choice["finish_reason"]
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
        let p = GenericOpenAiProvider::new(
            "deepseek".into(),
            "k".into(),
            "https://api.deepseek.com".into(),
            "deepseek-v4-pro".into(),
            30,
            3,
            1000,
            HashMap::new(),
        )
        .unwrap();
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
        )
        .unwrap();
        assert_eq!(p.retry_params(), (5, 2000));
    }
}
