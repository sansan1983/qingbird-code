//! DeepSeek API client — OpenAI 协议接入
//!
//! V0.1.0 只接 deepseek 单家。

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::http_client::{HttpClientConfig, HttpLlmClient};
use super::types::{ChatChunk, ChatRequest, ChatResponse, LlmProvider};
use crate::common::error::{EflowError, Result};

const CHAT_PATH: &str = "/chat/completions";
const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com";

pub struct DeepseekProvider {
    client: HttpLlmClient,
    default_model: String,
}

impl DeepseekProvider {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        default_model: String,
        timeout_secs: u64,
        max_retries: u8,
        retry_backoff_ms: u64,
    ) -> Result<Self> {
        let config = HttpClientConfig {
            base_url: base_url.unwrap_or_else(|| DEEPSEEK_BASE_URL.to_string()),
            api_key,
            timeout_secs,
            max_retries,
            retry_backoff_ms,
        };
        let client = HttpLlmClient::new(config)
            .map_err(|e| EflowError::Config(format!("HTTP client init failed: {}", e)))?;
        Ok(Self {
            client,
            default_model,
        })
    }
}

#[async_trait]
impl LlmProvider for DeepseekProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };
        self.client.chat(&model, CHAT_PATH, request).await
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };
        self.client.chat_stream(&model, CHAT_PATH, request).await
    }

    fn name(&self) -> &str {
        "deepseek"
    }
}
