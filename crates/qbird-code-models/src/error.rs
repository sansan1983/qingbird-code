use thiserror::Error;

#[derive(Error, Debug)]
pub enum EflowError {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("LLM Provider 错误: {0}")]
    LlmProvider(String),

    #[error("速率限制: {0}")]
    RateLimited(String),

    #[error("LLM 鉴权失败: {0}")]
    LlmAuthFailed(String),

    #[error("记忆系统错误: {0}")]
    Memory(String),

    #[error("工具错误: {0}")]
    Tool(String),

    #[error("风险升级: task={task_id}, reason={reason}")]
    RiskEscalated { task_id: String, reason: String },

    #[error("任务已取消: {0}")]
    TaskCancelled(String),

    #[error("Profile 不存在: {0}")]
    ProfileNotFound(String),

    #[error("Skill 不存在: {0}")]
    SkillNotFound(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    Serialization(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, EflowError>;
