use std::sync::Arc;

use qbird_code_models::Message;
use qbird_code_tools::ToolRegistry;

use crate::react_loop::{ReactLoop, ReactLoopConfig};

/// Subagent 角色
#[derive(Debug, Clone)]
pub enum SubagentRole {
    /// 只读探索
    CodeExplorer,
    /// 代码修改
    CodeWriter,
    /// 纯推理规划
    Planner,
    /// 通用
    Generalist,
}

impl SubagentRole {
    pub fn default_tools(&self) -> Vec<&'static str> {
        match self {
            SubagentRole::CodeExplorer => vec!["read_file", "search_code"],
            SubagentRole::CodeWriter => vec!["read_file", "write_file", "search_code"],
            SubagentRole::Planner => vec![],
            SubagentRole::Generalist => {
                vec!["read_file", "write_file", "search_code", "execute_command"]
            }
        }
    }
}

/// Subagent 配置
#[derive(Debug, Clone)]
pub struct SubagentConfig {
    pub name: String,
    pub role: SubagentRole,
    pub system_prompt: String,
    pub max_iterations: usize,
}

/// Subagent — 完整 ReAct 循环实例
pub struct Subagent {
    pub config: SubagentConfig,
    react_loop: ReactLoop,
}

impl Subagent {
    pub fn new(config: SubagentConfig) -> Self {
        let react_config = ReactLoopConfig {
            max_iterations: config.max_iterations,
            ..Default::default()
        };
        Self {
            config,
            react_loop: ReactLoop::new(react_config),
        }
    }

    /// 运行 subagent
    pub async fn run(
        &self,
        provider: &dyn qbird_code_infra::providers::Provider,
        http_client: &qbird_code_infra::http_client::HttpLlmClient,
        task: &str,
        tool_registry: &Arc<ToolRegistry>,
        tool_schemas: &[serde_json::Value],
    ) -> Result<String, qbird_code_models::EflowError> {
        let mut messages = vec![
            Message::system(&self.config.system_prompt),
            Message::user(task),
        ];

        let result = self
            .react_loop
            .run(
                provider,
                http_client,
                &mut messages,
                tool_schemas,
                tool_registry,
                Some(self.config.max_iterations),
                None,
            )
            .await?;

        Ok(result.content)
    }
}
