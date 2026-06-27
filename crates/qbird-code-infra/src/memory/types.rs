use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub path: String,
    pub scope: String,
    pub scope_id: Option<String>,
    pub r#type: String,
    pub body: String,
    pub fingerprint: String,
    pub last_indexed_at: i64,
}

#[derive(Debug, Clone)]
pub struct MemoryResult {
    pub entry: MemoryEntry,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct BudgetedReadResult {
    pub text: String,
    pub truncated: bool,
    pub total_tokens: usize,
    pub used_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct ContextMessage {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct CheckpointEvent {
    pub checkpoint_id: String,
    pub token_count: usize,
    pub reason: String,
    pub session_id: String,
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub model_limit: usize,
    pub current_usage: usize,
    pub reserved: usize,
}
