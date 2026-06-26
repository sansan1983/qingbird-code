pub mod types;

pub use types::{AgentResult, LoopState, ReactLoopConfig, TurnResult};

use std::sync::Arc;

use qbird_code_infra::providers::{ChatResponse, Provider, ProviderKind, RequestConfig};
use qbird_code_models::{Message, MessageRole, UsageStats};
use qbird_code_tools::ToolRegistry;

pub struct ReactLoop {
    pub config: ReactLoopConfig,
}

impl ReactLoop {
    pub fn new(config: ReactLoopConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(ReactLoopConfig::default())
    }
}
