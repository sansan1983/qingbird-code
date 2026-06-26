// Q&D: TODO fill

use serde::{Deserialize, Serialize};

/// Configuration for the ReAct loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactLoopConfig {
    pub max_iterations: usize,
}

impl Default for ReactLoopConfig {
    fn default() -> Self {
        Self { max_iterations: 10 }
    }
}

/// The main ReAct loop that orchestrates agent reasoning and tool execution.
pub struct ReactLoop {
    pub config: ReactLoopConfig,
}

impl ReactLoop {
    pub fn new(config: ReactLoopConfig) -> Self {
        Self { config }
    }
}
