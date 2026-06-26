use serde::{Deserialize, Serialize};

/// 消息角色
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// 单个工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具调用 ID（provider 返回）
    pub id: String,
    /// 工具名称
    #[serde(rename = "function")]
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    /// 函数/工具名称
    pub name: String,
    /// JSON 格式的参数
    pub arguments: String,
}

/// 统一消息类型
///
/// 兼容 OpenAI/Anthropic 格式，同时支持 DeepSeek thinking mode。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 消息角色
    pub role: MessageRole,
    /// 消息正文（可为空字符串，tool_calls 时可为空）
    pub content: String,
    /// DeepSeek thinking mode 返回的推理链内容
    /// 规则：
    ///   - assistant + 无 tool_calls: API 忽略，无需传回
    ///   - assistant + 有 tool_calls: 必须回传后续所有请求，否则 400
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// 工具调用列表（assistant role）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// 工具调用 ID（tool role，回传工具结果时用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// 工具调用名称（tool role）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    /// 返回角色的字符串表示（用于 API 请求序列化）
    pub fn role_str(&self) -> &str {
        match self.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        }
    }

    /// 创建 system 消息
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    /// 创建 user 消息
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    /// 创建 assistant 消息（带可选 thinking 内容）
    pub fn assistant(content: impl Into<String>, reasoning_content: Option<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            reasoning_content,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    /// 创建带 tool_calls 的 assistant 消息
    pub fn assistant_with_tools(
        content: impl Into<String>,
        reasoning_content: Option<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            reasoning_content,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            name: None,
        }
    }

    /// 创建 tool 结果消息
    pub fn tool_result(tool_call_id: String, name: String, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
            name: Some(name),
        }
    }

    /// 是否有工具调用
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls
            .as_ref()
            .map(|tc| !tc.is_empty())
            .unwrap_or(false)
    }
}

/// Token 用量统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageStats {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    /// DeepSeek 硬盘缓存命中 tokens
    pub cache_hit_tokens: u64,
    /// DeepSeek 硬盘缓存未命中 tokens
    pub cache_miss_tokens: u64,
}
