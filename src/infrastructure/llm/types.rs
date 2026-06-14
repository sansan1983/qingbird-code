use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::common::types::ModelTier;

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// 缓存控制
#[derive(Debug, Clone)]
pub struct CacheControlPoint {
    pub breakpoint_index: usize,
}

/// 聊天请求
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub cache_control: Option<CacheControlPoint>,
}

impl ChatRequest {
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            tools: None,
            temperature: 0.7,
            max_tokens: 4096,
            cache_control: None,
        }
    }

    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn with_cache(mut self, breakpoint_index: usize) -> Self {
        self.cache_control = Some(CacheControlPoint { breakpoint_index });
        self
    }
}

/// 聊天响应
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub usage: TokenUsage,
    pub finish_reason: String,
}

/// 流式消息块
#[derive(Debug, Clone)]
pub struct ChatChunk {
    pub content_delta: Option<String>,
    pub tool_call_delta: Option<ToolCallDelta>,
}

#[derive(Debug, Clone)]
pub struct ToolCallDelta {
    pub index: usize,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments_delta: Option<String>,
}

/// Token 使用量
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// LLM Provider trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// 非流式聊天
    async fn chat(&self, request: ChatRequest) -> crate::common::error::Result<ChatResponse>;

    /// 流式聊天
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> crate::common::error::Result<mpsc::Receiver<crate::common::error::Result<ChatChunk>>>;

    /// 是否支持前缀缓存
    fn supports_prefix_cache(&self) -> bool;

    /// Provider 名称
    fn name(&self) -> &str;
}

/// 把 ModelTier 映射成可读字符串（用于日志/事件）
pub fn tier_label(tier: ModelTier) -> &'static str {
    match tier {
        ModelTier::Strong => "strong",
        ModelTier::Medium => "medium",
        ModelTier::Light => "light",
    }
}
