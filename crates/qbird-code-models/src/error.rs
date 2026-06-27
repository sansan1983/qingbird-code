use thiserror::Error;

use rust_i18n::t;

#[derive(Error, Debug)]
pub enum EflowError {
    #[error("config error: {0}")]
    Config(String),

    #[error("LLM provider error: {0}")]
    LlmProvider(String),

    #[error("rate limited: {0}")]
    RateLimited(String),

    #[error("LLM auth failed: {0}")]
    LlmAuthFailed(String),

    #[error("memory system error: {0}")]
    Memory(String),

    #[error("tool error: {0}")]
    Tool(String),

    #[error("risk escalated: task={task_id}, reason={reason}")]
    RiskEscalated { task_id: String, reason: String },

    #[error("task cancelled: {0}")]
    TaskCancelled(String),

    #[error("profile not found: {0}")]
    ProfileNotFound(String),

    #[error("skill not found: {0}")]
    SkillNotFound(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, EflowError>;

impl EflowError {
    /// Localized user-facing message.
    ///
    /// For variants that have a matching `err_*` i18n key, this looks up the
    /// active locale via `rust_i18n::t!`. For internal-facing variants
    /// (`Io`, `Internal`, `Serialization`, `Tool`, `RateLimited`, `TaskCancelled`,
    /// `RiskEscalated`) it falls back to `Display` — these are normally surfaced
    /// through tracing logs (English) and not directly to end users.
    ///
    /// The `Display` strings above are in English to match the AGENTS.md rule
    /// that tracing logs stay in English; user-facing paths should call
    /// `user_message()` so the message tracks the active locale.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::Config(msg) => t!("err_config_load", msg = msg.as_str()).into_owned(),
            Self::LlmProvider(msg) => t!("err_llm_provider_init", msg = msg.as_str()).into_owned(),
            Self::Memory(msg) => t!("err_memory_init", msg = msg.as_str()).into_owned(),
            Self::ProfileNotFound(name) => {
                t!("err_profile_not_found", name = name.as_str()).into_owned()
            }
            Self::LlmAuthFailed(msg) => t!("err_llm_auth_failed", msg = msg.as_str()).into_owned(),
            Self::SkillNotFound(name) => {
                t!("err_skill_not_found", name = name.as_str()).into_owned()
            }
            Self::PermissionDenied(msg) => {
                t!("err_permission_denied", msg = msg.as_str()).into_owned()
            }
            _ => self.to_string(),
        }
    }
}
