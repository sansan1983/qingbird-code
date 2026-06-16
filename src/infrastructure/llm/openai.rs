use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio::sync::mpsc;

use super::types::{
    ChatChunk, ChatRequest, ChatResponse, LlmProvider, MessageRole, TokenUsage, ToolCall,
};
use super::{check_status, pick_model};
use crate::common::error::{EflowError, Result};
use rust_i18n::t;

const API_BASE: &str = "https://api.openai.com/v1";
const CHAT_PATH: &str = "/chat/completions";

pub struct OpenAiProvider {
    api_key: String,
    default_model: String,
    client: Client,
    max_retries: u8,
    retry_backoff_ms: u64,
    /// 可覆盖 endpoint base（OPENAI_BASE_URL env var），用于企业代理 / 第三方兼容服务
    /// 代码内部追加 `/chat/completions` —— 与官方 openai SDK 行为一致
    ///（v1.1 跨阶段：上一版把 base_url 当 full URL 用；改为 base 语义）
    base_url: String,
}

impl OpenAiProvider {
    #[must_use]
    pub fn new(api_key: String, default_model: String) -> Self {
        Self::with_options(api_key, default_model, 30, 3, 1000, None)
    }

    /// 注入 timeout + retry 参数（fix v1.1 Task A2）+ 可选 base_url
    #[must_use]
    pub fn with_options(
        api_key: String,
        default_model: String,
        timeout_secs: u64,
        max_retries: u8,
        retry_backoff_ms: u64,
        base_url: Option<String>,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("reqwest client build");
        Self {
            api_key,
            default_model,
            client,
            max_retries,
            retry_backoff_ms,
            base_url: base_url.unwrap_or_else(|| API_BASE.to_string()),
        }
    }

    /// Router 读取重试参数（Task A3 exponential backoff 用）
    #[must_use]
    pub fn retry_params(&self) -> (u8, u64) {
        (self.max_retries, self.retry_backoff_ms)
    }

    /// 构造 POST 请求（fix v1.0.3 R5 抽离）
    // v1.1 Task A4: OpenAI 不支持 Anthropic 风格的 cache_control 块。
    // 前缀缓存由 OpenAI 端自动管理（无需显式 breakpoint）。
    // 未来若启用 prompt_cache_key 头，在此接线。
    fn build_post(&self, body: &Value) -> reqwest::RequestBuilder {
        self.client
            .post(self.chat_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(body)
    }

    /// 完整 POST URL = base + /chat/completions（与官方 openai SDK 行为一致）
    fn chat_url(&self) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), CHAT_PATH)
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
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

    fn name(&self) -> &'static str {
        "openai"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_base_url_defaults_to_api_url_when_not_provided() {
        // v1.1 跨阶段: base 语义（与官方 openai SDK 行为一致）
        let p = OpenAiProvider::new("k".into(), "m".into());
        assert_eq!(p.base_url, "https://api.openai.com/v1");
        assert_eq!(p.chat_url(), "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn openai_base_url_overridden_when_provided() {
        // v1.1 跨阶段: Some(base) 覆盖默认值；base 是 base，不是 full URL
        let custom = "https://openrouter.ai/api/v1".to_string();
        let p =
            OpenAiProvider::with_options("k".into(), "m".into(), 30, 3, 1000, Some(custom.clone()));
        assert_eq!(p.base_url, custom);
        assert_eq!(
            p.chat_url(),
            "https://openrouter.ai/api/v1/chat/completions"
        );
    }

    #[test]
    fn openai_chat_url_trims_trailing_slash() {
        // base 带末尾 / 时也正确拼接
        let p = OpenAiProvider::with_options(
            "k".into(),
            "m".into(),
            30,
            3,
            1000,
            Some("https://proxy.example.com/v1/".to_string()),
        );
        assert_eq!(
            p.chat_url(),
            "https://proxy.example.com/v1/chat/completions"
        );
    }
}
