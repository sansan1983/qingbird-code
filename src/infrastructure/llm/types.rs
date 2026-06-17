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
            temperature: DEFAULT_TEMPERATURE,
            max_tokens: DEFAULT_MAX_TOKENS,
            cache_control: None,
        }
    }

    #[must_use]
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    #[must_use]
    pub fn with_cache(mut self, breakpoint_index: usize) -> Self {
        self.cache_control = Some(CacheControlPoint { breakpoint_index });
        self
    }
}

/// 默认采样温度（fix v1.0.3 M1 抽离）
pub const DEFAULT_TEMPERATURE: f32 = 0.7;
/// 默认最大输出 token 数（fix v1.0.3 M1 抽离）
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

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
///
/// **v1.3 起冻结**（spec A ADR-0011 + 稳定性约束 §4.7）：
/// - 方法签名不再破坏性变更
/// - 新增方法必须有默认实现
/// - 改语义必须 ADR + CHANGELOG
///
/// 原因：spec B 的 CLI 契约依赖 trait 稳定——GUI 套壳时如果 LLM 调用
/// 变体，CLI 契约层会 panic。
///
/// TODO(v1.4+): 加 .github/workflows/check-trait-stability.yml
/// 用 git diff 检查 trait 签名变化，CI 阻断破坏性 PR
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

    /// v1.3 候选：列出 provider 支持的模型
    ///
    /// 默认实现返回 `None`——表示"不支持自动拉取，调用方用 preset_models"。
    /// Generic adapter override 这个方法做实际 HTTP GET。
    fn list_models_endpoint(&self) -> Option<&str> {
        None
    }

    /// 重试参数 (max_retries, backoff_ms) — 供 Router 读取（fix v1.1 Task A3）
    /// 默认 (3, 1000)，具体 provider 可 override
    fn retry_params(&self) -> (u8, u64) {
        (3, 1000)
    }
}

/// 把 `ModelTier` 映射成可读字符串（用于日志/事件）
#[must_use]
pub fn tier_label(tier: ModelTier) -> &'static str {
    match tier {
        ModelTier::Strong => "strong",
        ModelTier::Medium => "medium",
        ModelTier::Light => "light",
    }
}

// =====================================================================
// v1.3 LLM 抽象扩展（spec A）
// =====================================================================

/// Provider 元数据，从 `~/.eflow/providers/{name}.yaml` 加载
///
/// v1.3 起 LLM provider **不**写死在 core crate——用户放 YAML 文件即可
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub display_name: String,
    pub protocol: ProtocolKind,
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
    #[serde(default)]
    pub preset_models: Vec<String>,
    #[serde(default)]
    pub list_models_endpoint: Option<String>,
    #[serde(default)]
    pub list_models: Vec<ModelEntry>,
    #[serde(default)]
    pub extra_config: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolKind {
    OpenaiCompatible,
    AnthropicCompatible,
}

/// model id → endpoint path 映射（OpenCode Go 场景）
#[derive(Debug, Clone, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    #[serde(default)]
    pub endpoint: Option<String>,
}

fn default_timeout_secs() -> u64 {
    30
}
fn default_max_retries() -> u8 {
    3
}
fn default_retry_backoff_ms() -> u64 {
    1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_kind_serde_roundtrip_openai() {
        let json = serde_json::to_string(&ProtocolKind::OpenaiCompatible).unwrap();
        assert_eq!(json, "\"openai_compatible\"");
        let back: ProtocolKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ProtocolKind::OpenaiCompatible);
    }

    #[test]
    fn protocol_kind_serde_roundtrip_anthropic() {
        let json = serde_json::to_string(&ProtocolKind::AnthropicCompatible).unwrap();
        assert_eq!(json, "\"anthropic_compatible\"");
        let back: ProtocolKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ProtocolKind::AnthropicCompatible);
    }

    #[test]
    fn model_entry_endpoint_optional() {
        let yaml = "id: deepseek-v4-pro\n";
        let entry: ModelEntry = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(entry.id, "deepseek-v4-pro");
        assert_eq!(entry.endpoint, None);
    }

    #[test]
    fn model_entry_with_endpoint() {
        let yaml = "id: glm-5.1\nendpoint: /v1/chat/completions\n";
        let entry: ModelEntry = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(entry.id, "glm-5.1");
        assert_eq!(entry.endpoint, Some("/v1/chat/completions".to_string()));
    }

    #[test]
    fn provider_config_defaults() {
        let yaml = r#"
id: test
display_name: Test
protocol: openai_compatible
base_url: https://api.test.com
api_key: "key"
default_model: test-model
"#;
        let cfg: ProviderConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.timeout_secs, 30); // 默认
        assert_eq!(cfg.max_retries, 3); // 默认
        assert_eq!(cfg.retry_backoff_ms, 1000); // 默认
        assert!(cfg.preset_models.is_empty());
        assert!(cfg.list_models_endpoint.is_none());
    }
}
