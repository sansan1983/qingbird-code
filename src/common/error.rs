use thiserror::Error;

#[derive(Error, Debug)]
pub enum EflowError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("LLM provider error: {0}")]
    LlmProvider(String),

    #[error("LLM rate limit exceeded for provider {0}")]
    RateLimited(String),

    #[error("LLM authentication failed for provider {0}")]
    LlmAuthFailed(String),

    #[error("memory error: {0}")]
    Memory(String),

    #[error("tool execution error: {0}")]
    Tool(String),

    #[error("risk escalation: task {task_id}, reason: {reason}")]
    RiskEscalated { task_id: String, reason: String },

    #[error("task cancelled: {0}")]
    TaskCancelled(String),

    #[error("profile not found: {0}")]
    ProfileNotFound(String),

    #[error("skill not found: {0}")]
    SkillNotFound(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, EflowError>;
