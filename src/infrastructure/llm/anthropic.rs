use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio::sync::mpsc;

use super::types::{ChatChunk, ChatRequest, ChatResponse, LlmProvider, MessageRole, TokenUsage};
use super::{check_status, pick_model};
use crate::common::error::{EflowError, Result};
use rust_i18n::t;

const API_BASE: &str = "https://api.anthropic.com";
const MESSAGES_PATH: &str = "/v1/messages";

pub struct AnthropicProvider {
    api_key: String,
    default_model: String,
    client: Client,
    max_retries: u8,
    retry_backoff_ms: u64,
    /// 可覆盖 endpoint base（ANTHROPIC_BASE_URL env var），用于企业代理 / 第三方兼容服务
    /// 代码内部追加 `/v1/messages` —— 与官方 anthropic SDK 行为一致
    ///（v1.1 跨阶段：上一版把 base_url 当 full URL 用，minimaxi 等代理 404；改为 base 语义）
    base_url: String,
}

impl AnthropicProvider {
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

    /// 构造 POST 请求（共用 chat + chat_stream，fix v1.0.3 R5 抽离）
    fn build_post(&self, body: &Value) -> reqwest::RequestBuilder {
        self.client
            .post(self.messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(body)
    }

    /// 完整 POST URL = base + /v1/messages（与官方 anthropic SDK 行为一致）
    fn messages_url(&self) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), MESSAGES_PATH)
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
impl LlmProvider for AnthropicProvider {
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

    async fn chat_stream(&self, request: ChatRequest) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        let api_key = self.api_key.clone();
        let default_model = self.default_model.clone();
        let client = self.client.clone();
        let messages_url = self.messages_url();

        let model = pick_model(&default_model, &request.model);
        let mut body = self.build_body(&ChatRequest {
            model: model.clone(),
            ..request
        });
        body["stream"] = serde_json::json!(true);

        let (tx, rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let response = match client
                .post(&messages_url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(Err(EflowError::LlmProvider(format!("{e}")))).await;
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
                                    let content_delta = json["delta"]["text"]
                                        .as_str()
                                        .map(std::string::ToString::to_string);
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

    fn name(&self) -> &'static str {
        "anthropic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::llm::Message;

    #[tokio::test]
    async fn anthropic_provider_uses_configured_timeout() {
        // v1.1 Task A2: 验证 with_options 注入的 timeout 真的生效。
        // 0 秒 timeout → 任何 HTTP 请求立即失败 → 应被归类为 LlmProvider 错误。
        // 用 127.0.0.1:1 (TCP 保留端口) 触发连接失败，但更快的方式是让 reqwest client
        // 自身在 0s timeout 下连不出去。
        let p = AnthropicProvider::with_options(
            "sk-test".into(),
            "claude-sonnet-4-6".into(),
            0, // 0 秒 = 立即超时
            1,
            100,
            None, // base_url 默认
        );
        let req = ChatRequest::new("", vec![Message::user("hi")]);
        let result = p.chat(req).await;
        assert!(matches!(result, Err(EflowError::LlmProvider(_))));
    }

    #[test]
    fn anthropic_body_includes_cache_control_on_system() {
        // v1.1 Task A4: 验证 build_body 在 system 块加 cache_control: ephemeral
        let p = AnthropicProvider::new("k".into(), "m".into());
        let req = ChatRequest::new(
            "claude-sonnet-4-6",
            vec![Message::system("you are eflow"), Message::user("hi")],
        );
        let body = p.build_body(&req);
        assert_eq!(body["system"][0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn anthropic_base_url_defaults_to_api_url_when_not_provided() {
        // v1.1 跨阶段: base_url 字段默认值是 api.anthropic.com（base 而非 full URL）
        let p = AnthropicProvider::new("k".into(), "m".into());
        assert_eq!(p.base_url, "https://api.anthropic.com");
    }

    #[test]
    fn anthropic_base_url_overridden_when_provided() {
        // v1.1 跨阶段: Some(url) 覆盖默认值（企业代理 / 第三方兼容服务）
        let custom = "https://api.minimaxi.com/anthropic".to_string();
        let p = AnthropicProvider::with_options(
            "k".into(),
            "m".into(),
            30,
            3,
            1000,
            Some(custom.clone()),
        );
        assert_eq!(p.base_url, custom);
    }

    #[test]
    fn anthropic_messages_url_appends_v1_messages() {
        // v1.1 跨阶段: base + /v1/messages 才是完整 POST URL（与 SDK 行为一致）
        // 关键：minimaxi 等代理只接受完整 /v1/messages 路径，POST / 会 404
        let p = AnthropicProvider::new("k".into(), "m".into());
        assert_eq!(p.messages_url(), "https://api.anthropic.com/v1/messages");

        let custom = AnthropicProvider::with_options(
            "k".into(),
            "m".into(),
            30,
            3,
            1000,
            Some("https://api.minimaxi.com/anthropic".into()),
        );
        assert_eq!(
            custom.messages_url(),
            "https://api.minimaxi.com/anthropic/v1/messages"
        );

        // 尾部带 / 也不重复
        let trailing = AnthropicProvider::with_options(
            "k".into(),
            "m".into(),
            30,
            3,
            1000,
            Some("https://api.example.com/".into()),
        );
        assert_eq!(
            trailing.messages_url(),
            "https://api.example.com/v1/messages"
        );
    }
}
