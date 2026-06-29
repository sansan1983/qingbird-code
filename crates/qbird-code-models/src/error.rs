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

    #[error("session not found: {id}")]
    SessionNotFound { id: String },

    #[error("session prefix {prefix} is ambiguous ({count} matches)")]
    SessionAmbiguous { prefix: String, count: usize },
}

pub type Result<T> = std::result::Result<T, EflowError>;

impl EflowError {
    /// Localized user-facing message.
    ///
    /// Every variant goes through a matching `err_*` i18n key, looked up via
    /// `rust_i18n::t!` in the active locale. The `Display` strings on the
    /// enum variants are kept in English to match the AGENTS.md rule that
    /// tracing logs stay in English; user-facing paths should call
    /// `user_message()` so the message tracks the active locale.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::Config(msg) => t!("err_config_load", msg = msg.as_str()).into_owned(),
            Self::LlmProvider(msg) => t!("err_llm_provider_init", msg = msg.as_str()).into_owned(),
            Self::RateLimited(msg) => t!("err_rate_limited_msg", msg = msg.as_str()).into_owned(),
            Self::LlmAuthFailed(msg) => t!("err_llm_auth_failed", msg = msg.as_str()).into_owned(),
            Self::Memory(msg) => t!("err_memory_init", msg = msg.as_str()).into_owned(),
            Self::Tool(msg) => t!("err_tool", msg = msg.as_str()).into_owned(),
            Self::RiskEscalated { task_id, reason } => t!(
                "err_risk_escalated",
                task_id = task_id.as_str(),
                reason = reason.as_str()
            )
            .into_owned(),
            Self::TaskCancelled(id) => t!("err_task_cancelled", id = id.as_str()).into_owned(),
            Self::ProfileNotFound(name) => {
                t!("err_profile_not_found", name = name.as_str()).into_owned()
            }
            Self::SkillNotFound(name) => {
                t!("err_skill_not_found", name = name.as_str()).into_owned()
            }
            Self::PermissionDenied(msg) => {
                t!("err_permission_denied", msg = msg.as_str()).into_owned()
            }
            Self::Io(err) => t!("err_io", msg = &err.to_string()).into_owned(),
            Self::Serialization(msg) => t!("err_serialization", msg = msg.as_str()).into_owned(),
            Self::Internal(msg) => t!("err_internal", msg = msg.as_str()).into_owned(),
            Self::SessionNotFound { id } => {
                t!("err_session_not_found", id = id.as_str()).into_owned()
            }
            Self::SessionAmbiguous { prefix, count } => t!(
                "err_session_ambiguous",
                prefix = prefix.as_str(),
                count = count
            )
            .into_owned(),
        }
    }
}
