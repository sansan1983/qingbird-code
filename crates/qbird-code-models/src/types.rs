use serde::{Deserialize, Serialize};

// ========== 风险等级 ==========

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum RiskLevel {
    #[default]
    L0, // 只读操作 — 自动执行
    L1, // 文件写入 — 自动执行 + 安全检查
    L2, // 系统命令 — 沙箱隔离执行
    L3, // 高危操作 — 人工确认
}

// ========== 重试策略 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u8,
    pub backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff_ms: 1000,
        }
    }
}

// ========== 角色/能力 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    FileAssistant,
    CodeAssistant,
    DataAnalyst,
    Generalist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    ReadFile,
    WriteFile,
    ExecuteCommand,
    SearchCode,
    WebFetch,
    LlmReasoning,
}

// ========== 权限 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSet {
    pub allowed_paths: Vec<String>,
    pub allowed_commands: Vec<String>,
    pub max_file_size_bytes: u64,
    pub network_enabled: bool,
}

impl Default for PermissionSet {
    fn default() -> Self {
        Self {
            allowed_paths: vec![],
            allowed_commands: vec![],
            max_file_size_bytes: DEFAULT_MAX_FILE_SIZE,
            network_enabled: false,
        }
    }
}

/// 默认文件大小上限 10MB（fix v1.0.3 M2 抽离）
pub const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

// ========== 记忆类型 ==========

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryCategory {
    TaskResult,
    Decision,
    Feedback,
    UserPreference,
    LearnedPattern,
    ManualNote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Importance {
    Low,
    Normal,
    High,
    Pinned,
}
