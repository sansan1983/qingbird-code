use std::collections::HashSet;
use std::path::PathBuf;

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
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub backoff_multiplier: f64,
    pub max_backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1000,
            backoff_multiplier: 2.0,
            max_backoff_ms: 30_000,
        }
    }
}

impl RetryPolicy {
    /// Calculate the backoff delay for a given retry attempt (0-indexed).
    /// Capped at `max_backoff_ms`.
    #[must_use]
    pub fn backoff_for_attempt(&self, attempt: u32) -> u64 {
        let base = self.initial_backoff_ms as f64;
        let delay = base * self.backoff_multiplier.powi(attempt as i32);
        (delay as u64).min(self.max_backoff_ms)
    }
}

// ========== 角色 / 能力 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub permissions: PermissionSet,
}

impl Role {
    #[must_use]
    pub fn new(name: impl Into<String>, permissions: PermissionSet) -> Self {
        Self {
            name: name.into(),
            permissions,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub description: String,
}

impl Capability {
    #[must_use]
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

// ========== 权限 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSet {
    /// Empty set = allow all tools (whitelist disabled).
    pub allowed_tools: HashSet<String>,
    /// Empty set = allow all paths (path check disabled).
    pub allowed_paths: HashSet<PathBuf>,
    /// Maximum risk level allowed; higher attempts are blocked.
    pub max_risk: RiskLevel,
}

impl Default for PermissionSet {
    fn default() -> Self {
        Self {
            allowed_tools: HashSet::new(),
            allowed_paths: HashSet::new(),
            max_risk: RiskLevel::L3,
        }
    }
}

impl PermissionSet {
    /// `true` when the tool is allowed (whitelist empty or contains it).
    #[must_use]
    pub fn allows_tool(&self, tool_name: &str) -> bool {
        self.allowed_tools.is_empty() || self.allowed_tools.contains(tool_name)
    }

    /// `true` when the path is allowed (whitelist empty or contains it).
    #[must_use]
    pub fn allows_path(&self, path: &PathBuf) -> bool {
        self.allowed_paths.is_empty() || self.allowed_paths.contains(path)
    }

    /// `true` when the given risk level is at or below `max_risk`.
    #[must_use]
    pub fn allows_risk(&self, risk: RiskLevel) -> bool {
        risk <= self.max_risk
    }
}

// ========== 记忆类型 ==========

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryCategory {
    /// Project-scoped memory shared across sessions for a project.
    Project,
    /// User-scoped memory persisted across all projects.
    User,
    /// Re-usable code snippet or template.
    Snippet,
    /// Tool invocation history / output.
    Tool,
}

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum Importance {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}
