//! `SubagentExecutor` — 子 agent 生命周期管理。
//!
//! 给 LLM 派发子任务时调用 `spawn_child_with_provider`：
//! 1. 查 profile（不在则返回 `SubagentProfileNotFound`）
//! 2. 根据 `tool_policy` 构造独立工具集
//! 3. 创建独立 ReactLoop 实例
//! 4. 跑完返回 `ChildRecord`
//!
//! 设计参考 `F:\AI\Kun\kun\src\delegation\child-agent-executor.ts`。
//! v0.3.1 简化：同步等子 agent 完成（detach 留 v0.4）。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::providers::Provider;
use qbird_code_models::{EflowError, Message, UsageStats};
use qbird_code_tools::ToolRegistry;

use crate::react_loop::{ReactLoop, ReactLoopConfig};

use super::profile::{SubagentProfile, ToolPolicy};

#[derive(Clone, Default)]
pub struct SubagentSpawnHints {
    pub parent_session_id: Option<String>,
    pub parent_turn_id: Option<String>,
    pub detached: bool,
    pub priority: SpawnPriority,
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
    profiles: HashMap<String, SubagentProfile>,
    base_config: ReactLoopConfig,
    tool_registry: Arc<ToolRegistry>,
}

pub struct SubagentExecutorBuilder {
    profiles: Option<HashMap<String, SubagentProfile>>,
    base_config: Option<ReactLoopConfig>,
    tool_registry: Option<Arc<ToolRegistry>>,
}

impl SubagentExecutor {
    pub fn builder() -> SubagentExecutorBuilder {
        SubagentExecutorBuilder {
            profiles: None,
            base_config: None,
            tool_registry: None,
        }
    }

    pub fn validate_profile(&self, name: &str) -> Result<&SubagentProfile, EflowError> {
        self.profiles
            .get(name)
            .ok_or_else(|| EflowError::SubagentProfileNotFound {
                name: name.to_string(),
            })
    }

    pub fn list_profile_names(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }

    pub async fn spawn_child_with_provider(
        &self,
        profile_name: &str,
        prompt: &str,
        hints: &SubagentSpawnHints,
        provider: &dyn Provider,
        http_client: &HttpLlmClient,
    ) -> Result<ChildRecord, EflowError> {
        let profile = self.validate_profile(profile_name)?.clone();
        let child_id = uuid::Uuid::new_v4().to_string();
        let started = Instant::now();

        if let Some(cb) = &hints.on_event {
            cb(ChildEvent::Started {
                child_id: child_id.clone(),
            });
        }

        let max_iter = profile
            .max_iterations
            .unwrap_or(self.base_config.max_iterations);
        let child_config = ReactLoopConfig {
            max_iterations: max_iter,
            model: profile
                .model
                .clone()
                .unwrap_or_else(|| self.base_config.model.clone()),
            ..self.base_config.clone()
        };

        let system_prompt = format!(
            "{}\n\n[父代理任务]\n{}\n\n[约束]\n- 你是子代理，独立完成任务\n- 完成后简洁汇报结果",
            profile.prompt_preamble, prompt
        );
        let mut messages = vec![Message::system(&system_prompt), Message::user(prompt)];

        let child_tool_schemas = match profile.tool_policy {
            ToolPolicy::ReadOnly => self.read_only_tool_schemas(),
            ToolPolicy::Inherit => self.base_tool_schemas(),
        };

        let react_loop = ReactLoop::new(child_config);
        let result = react_loop
            .run(
                provider,
                http_client,
                &mut messages,
                &child_tool_schemas,
                &self.tool_registry,
                Some(max_iter),
                None,
                None,
            )
            .await;

        let duration_ms = started.elapsed().as_millis() as u64;

        match result {
            Ok(agent_result) => {
                let record = ChildRecord {
                    child_id,
                    status: ChildStatus::Completed,
                    summary: agent_result.content,
                    usage: agent_result.usage,
                    profile: profile_name.to_string(),
                    tool_policy: profile.tool_policy,
                    duration_ms,
                };
                if let Some(cb) = &hints.on_event {
                    cb(ChildEvent::Completed {
                        summary: record.summary.clone(),
                        usage: record.usage.clone(),
                    });
                }
                Ok(record)
            }
            Err(e) => {
                let err_msg = format!("{}", e);
                if let Some(cb) = &hints.on_event {
                    cb(ChildEvent::Failed {
                        error: err_msg.clone(),
                    });
                }
                Ok(ChildRecord {
                    child_id,
                    status: ChildStatus::Failed,
                    summary: err_msg,
                    usage: UsageStats::default(),
                    profile: profile_name.to_string(),
                    tool_policy: profile.tool_policy,
                    duration_ms,
                })
            }
        }
    }

    fn read_only_tool_schemas(&self) -> Vec<serde_json::Value> {
        let read_only = SubagentProfile::read_only_tool_names();
        self.tool_registry
            .definitions()
            .into_iter()
            .filter(|d| read_only.contains(&d.name.as_str()))
            .map(|d| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": d.name,
                        "description": d.description,
                        "parameters": d.parameters,
                    }
                })
            })
            .collect()
    }

    fn base_tool_schemas(&self) -> Vec<serde_json::Value> {
        self.tool_registry
            .definitions()
            .into_iter()
            .map(|d| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": d.name,
                        "description": d.description,
                        "parameters": d.parameters,
                    }
                })
            })
            .collect()
    }
}

impl SubagentExecutorBuilder {
    pub fn profiles(mut self, profiles: HashMap<String, SubagentProfile>) -> Self {
        self.profiles = Some(profiles);
        self
    }

    pub fn base_config(mut self, config: ReactLoopConfig) -> Self {
        self.base_config = Some(config);
        self
    }

    pub fn tool_registry(mut self, registry: Arc<ToolRegistry>) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    pub fn build(self) -> Result<SubagentExecutor, EflowError> {
        Ok(SubagentExecutor {
            profiles: self
                .profiles
                .ok_or_else(|| EflowError::Internal("SubagentExecutor: profiles 必填".into()))?,
            base_config: self
                .base_config
                .ok_or_else(|| EflowError::Internal("SubagentExecutor: base_config 必填".into()))?,
            tool_registry: self.tool_registry.ok_or_else(|| {
                EflowError::Internal("SubagentExecutor: tool_registry 必填".into())
            })?,
        })
    }
}
