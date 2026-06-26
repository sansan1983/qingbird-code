// Q&D: TODO fill

/// The role of a subagent in the agent system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubagentRole {
    Decisioner,
    Executor,
    Feedbacker,
}

/// Configuration for a subagent.
#[derive(Debug, Clone)]
pub struct SubagentConfig {
    pub role: SubagentRole,
    pub max_retries: usize,
}

impl Default for SubagentConfig {
    fn default() -> Self {
        Self {
            role: SubagentRole::Executor,
            max_retries: 3,
        }
    }
}

/// A subagent that can be spawned to perform independent tasks.
pub struct Subagent {
    pub config: SubagentConfig,
}

impl Subagent {
    pub fn new(config: SubagentConfig) -> Self {
        Self { config }
    }
}
