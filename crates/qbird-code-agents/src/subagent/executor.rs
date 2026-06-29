//! `SubagentExecutor` — 子 agent 生命周期管理（PR-B2 完整实现）

use std::sync::Arc;

use qbird_code_models::{EflowError, UsageStats};

use super::profile::ToolPolicy;

#[derive(Clone, Default)]
pub struct SubagentSpawnHints {
    pub parent_session_id: Option<String>,
    pub parent_turn_id: Option<String>,
    /// v0.3.1 固定 false
    pub detached: bool,
    /// v0.3.1 固定 Normal
    pub priority: SpawnPriority,
    /// v0.4+ 事件回调
    pub on_event: Option<Arc<dyn Fn(ChildEvent) + Send + Sync>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SpawnPriority {
    #[default]
    Normal,
    Low,
}

#[derive(Debug, Clone)]
pub enum ChildEvent {
    Started { child_id: String },
    Completed { summary: String, usage: UsageStats },
    Failed { error: String },
}

#[derive(Debug, Clone)]
pub struct ChildRecord {
    pub child_id: String,
    pub status: ChildStatus,
    pub summary: String,
    pub usage: UsageStats,
    pub profile: String,
    pub tool_policy: ToolPolicy,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildStatus {
    Completed,
    Failed,
}

pub struct SubagentExecutor {
    pub(crate) _placeholder: (),
}

impl SubagentExecutor {
    pub fn placeholder() -> Self {
        Self { _placeholder: () }
    }

    /// 临时：返回 IO error 让编译过
    pub fn list_profile_names(&self) -> Vec<String> {
        vec![]
    }
    pub fn validate_profile(
        &self,
        _name: &str,
    ) -> Result<&super::profile::SubagentProfile, EflowError> {
        Err(EflowError::Internal("placeholder".into()))
    }
}
