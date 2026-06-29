//! `DelegateTaskTool` — 让 LLM 主动派发子任务给 subagent。
//!
//! 设计参考 `F:\AI\Kun\kun\src\adapters\tool\delegation-tool-provider.ts`。
//! v0.3.1 简化：同步等子 agent 完成；detach 留 v0.4。
//!
//! 放在 `qbird_code_agents` 而不是 `qbird_code_tools`，因为它依赖
//! `SubagentExecutor` 和 `HttpLlmClient`（都在 agents/infra），保持 crate
//! 依赖方向 tools ← agents 单向。

use std::sync::Arc;

use async_trait::async_trait;
use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::providers::Provider;
use qbird_code_models::{EflowError, Result, RiskLevel};
use qbird_code_tools::{Tool, ToolDefinition, ToolOutput};
use rust_i18n::t;
use serde_json::json;

use crate::subagent::{ChildStatus, SubagentExecutorTrait, SubagentSpawnHints};

pub struct DelegateTaskTool {
    executor: Arc<dyn SubagentExecutorTrait>,
}

impl DelegateTaskTool {
    pub fn new(executor: Arc<dyn SubagentExecutorTrait>) -> Self {
        Self { executor }
    }

    pub fn executor(&self) -> &Arc<dyn SubagentExecutorTrait> {
        &self.executor
    }

    pub async fn execute_with_provider(
        &self,
        params: serde_json::Value,
        provider: &dyn Provider,
        http_client: &HttpLlmClient,
    ) -> Result<ToolOutput> {
        let label = params
            .get("label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                EflowError::Tool(t!("err_tool_missing_param", name = "label").to_string())
            })?;
        let prompt = params
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                EflowError::Tool(t!("err_tool_missing_param", name = "prompt").to_string())
            })?;
        let profile_name = params
            .get("profile")
            .and_then(|v| v.as_str())
            .unwrap_or("general");

        self.executor.validate_profile(profile_name)?;

        let hints = SubagentSpawnHints::default();
        let record = self
            .executor
            .spawn_child_with_provider(profile_name, prompt, &hints, provider, http_client)
            .await?;

        let output_json = json!({
            "child_id": record.child_id,
            "label": label,
            "status": format!("{:?}", record.status),
            "summary": record.summary,
            "profile": record.profile,
            "tool_policy": format!("{:?}", record.tool_policy),
            "duration_ms": record.duration_ms,
        });

        Ok(ToolOutput {
            success: record.status == ChildStatus::Completed,
            content: serde_json::to_string_pretty(&output_json).unwrap_or_default(),
            metadata: Some(output_json),
        })
    }
}

#[async_trait]
impl Tool for DelegateTaskTool {
    fn definition(&self) -> ToolDefinition {
        let profiles = self.executor.list_profile_names();
        let profile_names_str = profiles.join(", ");
        ToolDefinition {
            name: "delegate_task".to_string(),
            description: format!(
                "{}\n\n可用 profiles: {}。profile 省略时默认 'general'。",
                t!("tool_delegate_task_description"),
                profile_names_str
            ),
            parameters: json!({
                "type": "object",
                "properties": {
                    "label": {"type": "string", "description": "2-4 词子任务标题，UI 显示"},
                    "prompt": {"type": "string", "description": "交给子代理的具体任务"},
                    "workspace": {"type": "string", "description": "子代理工作目录（可选）"},
                    "model": {"type": "string", "description": "覆盖子代理模型（可选，v0.3.1 暂未实现）"},
                    "profile": {
                        "type": "string",
                        "enum": profiles,
                        "description": "子代理角色（默认 general）"
                    }
                },
                "required": ["prompt", "label"],
                "additionalProperties": false
            }),
            risk_level: RiskLevel::L2,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> Result<ToolOutput> {
        Err(EflowError::Internal(
            "DelegateTaskTool 必须通过 execute_with_provider 调用".into(),
        ))
    }
}
