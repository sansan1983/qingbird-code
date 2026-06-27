pub mod anthropic;
pub mod deepseek;
pub mod deepseek_anthropic;
pub mod ollama;
pub mod openai;
pub mod stream;

pub use anthropic::AnthropicProvider;
pub use deepseek::DeepseekProvider;
pub use deepseek_anthropic::DeepseekAnthropicProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;

use async_trait::async_trait;
use qbird_code_models::{Message, Result, UsageStats};
use serde::{Deserialize, Serialize};

use crate::http_client::HttpLlmClient;

/// Provider 标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    DeepSeek,
    OpenAI,
    Anthropic,
    Ollama,
}

/// 协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolKind {
    /// OpenAI 兼容协议 — endpoint: /chat/completions
    OpenAICompatible,
    /// Anthropic 原生协议 — endpoint: /messages
    Anthropic,
}

/// 请求配置
#[derive(Debug, Clone)]
pub struct RequestConfig {
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    pub stream: bool,
    /// thinking mode 开关
    pub thinking_enabled: bool,
    /// thinking effort: "high" | "max"
    pub thinking_effort: Option<String>,
    /// 工具定义（JSON Schema）
    pub tools: Vec<serde_json::Value>,
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: false,
            thinking_enabled: true,
            thinking_effort: Some("high".into()),
            tools: vec![],
        }
    }
}

/// 统一聊天响应
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<serde_json::Value>>,
    pub finish_reason: Option<String>,
    pub usage: UsageStats,
}

/// Provider trait — 所有 LLM Provider 实现此 trait
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider 标识
    fn kind(&self) -> ProviderKind;

    /// 协议类型
    fn protocol(&self) -> ProtocolKind;

    /// 当前模型名称
    fn model(&self) -> &str;

    /// 基础 URL
    fn base_url(&self) -> &str;

    /// 完整 API endpoint
    fn endpoint(&self) -> String {
        match self.protocol() {
            ProtocolKind::OpenAICompatible => format!("{}/chat/completions", self.base_url()),
            ProtocolKind::Anthropic => format!("{}/messages", self.base_url()),
        }
    }

    /// 构建 HTTP 请求体 (JSON)
    fn build_request_body(&self, messages: &[Message], config: &RequestConfig)
    -> serde_json::Value;

    /// 解析响应体为统一 ChatResponse
    async fn parse_response(&self, body: &serde_json::Value) -> Result<ChatResponse>;

    /// 构建 HTTP 请求 headers
    fn build_headers(&self) -> std::collections::HashMap<String, String>;

    /// 发送流式请求。返回完整响应（默认回退到非流式）
    async fn stream(
        &self,
        http_client: &HttpLlmClient,
        messages: &[Message],
        config: &RequestConfig,
    ) -> Result<ChatResponse>
    where
        Self: Sized,
    {
        let mut req_config = config.clone();
        req_config.stream = true;
        let body = self.build_request_body(messages, &req_config);
        let response_json = http_client.send(self, &body).await?;
        self.parse_response(&response_json).await
    }
}
