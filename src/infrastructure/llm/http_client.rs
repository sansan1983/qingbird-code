//! HTTP client for OpenAI-compatible API (V0.1.0 deepseek-only)

use std::time::Duration;

use reqwest::Client;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use super::types::{ChatChunk, ChatRequest, ChatResponse, MessageRole, TokenUsage};
use crate::common::error::{EflowError, Result};

/// HTTP 客户端配置
pub struct HttpClientConfig {
    pub base_url: String,
    pub api_key: String,
    pub timeout_secs: u64,
    pub max_retries: u8,
    pub retry_backoff_ms: u64,
}

/// HTTP 客户端（OpenAI 兼容）
pub struct HttpLlmClient {
    client: Client,
    config: HttpClientConfig,
}

impl HttpLlmClient {
    /// 创建新的 HTTP 客户端
    pub fn new(config: HttpClientConfig) -> std::result::Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client, config })
    }

    /// 非流式聊天
    pub async fn chat(
        &self,
        model: &str,
        path: &str,
        request: ChatRequest,
    ) -> Result<ChatResponse> {
        let url = format!("{}{}", self.config.base_url.trim_end_matches('/'), path);
        let body = Self::build_request_body(model, &request);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| EflowError::LlmProvider(format!("HTTP request failed: {}", e)))?;
        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            return Err(EflowError::LlmProvider(format!(
                "API error ({}): {}",
                status, body_text
            )));
        }
        // Parse as generic JSON first, then extract fields
        let json: Value = response
            .json()
            .await
            .map_err(|e| EflowError::LlmProvider(format!("JSON parse failed: {}", e)))?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let input_tokens = json["usage"]["prompt_tokens"]
            .as_u64()
            .or_else(|| json["usage"]["input_tokens"].as_u64())
            .unwrap_or(0) as u32;
        let output_tokens = json["usage"]["completion_tokens"]
            .as_u64()
            .or_else(|| json["usage"]["output_tokens"].as_u64())
            .unwrap_or(0) as u32;

        let finish_reason = json["choices"][0]["finish_reason"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(ChatResponse {
            content,
            tool_calls: None,
            usage: TokenUsage {
                input_tokens,
                output_tokens,
            },
            finish_reason,
        })
    }

    /// 流式聊天（V0.1.0 尚未实现）
    pub async fn chat_stream(
        &self,
        _model: &str,
        _path: &str,
        _request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        Err(EflowError::LlmProvider(
            "streaming not yet supported".into(),
        ))
    }

    /// 构建 OpenAI 兼容的请求体
    fn build_request_body(model: &str, request: &ChatRequest) -> Value {
        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|m| {
                let role_str = match m.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                };
                json!({ "role": role_str, "content": m.content })
            })
            .collect();

        json!({
            "model": model,
            "messages": messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
        })
    }
}
