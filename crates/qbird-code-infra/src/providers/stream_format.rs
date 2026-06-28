use serde::{Deserialize, Serialize};

use super::ChatResponse;

/// SSE format variant — each provider declares which one it uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamFormat {
    /// OpenAI-compatible SSE: `data: {"choices":[{"delta":...}]}`
    OpenAICompatible,
    /// Anthropic SSE: `event: content_block_delta\ndata: {"delta":{...}}`
    Anthropic,
}

/// Incremental tool-call delta received during streaming.
#[derive(Debug, Clone, Default)]
pub struct ToolCallDelta {
    /// Tool call ID (set once at first appearance).
    pub id: Option<String>,
    /// Function name (set once at first appearance).
    pub name: Option<String>,
    /// Incremental argument fragment (appended each chunk).
    pub arguments_delta: String,
}

/// A single event emitted by the streaming parser.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Incremental text content delta.
    TextDelta(String),
    /// Incremental reasoning/thinking content delta.
    ReasoningDelta(String),
    /// Incremental tool-call update (index → delta).
    ToolCallDelta { index: usize, delta: ToolCallDelta },
    /// Stream complete — carries the fully assembled ChatResponse.
    Done(ChatResponse),
    /// Non-fatal error; caller should log and continue or fall back.
    Error(String),
}
