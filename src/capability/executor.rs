use chrono::Utc;
use std::sync::Arc;

use super::blackboard::Blackboard;
use crate::capability::tools::ToolRegistry;
use crate::common::error::Result;
use crate::common::types::{
    ActionRecord, ActionResult, IntentType, ModelTier, RiskLevel, TaskStep, ToolCallSummary,
};
use crate::infrastructure::llm::cache::cache_key_for_step;
use crate::infrastructure::llm::{ChatRequest, LlmRouter, Message};
use rust_i18n::t;

/// Executor — 步骤执行
pub struct Executor {
    llm: Arc<tokio::sync::Mutex<LlmRouter>>,
    tools: Arc<ToolRegistry>,
}

impl Executor {
    pub fn new(llm: Arc<tokio::sync::Mutex<LlmRouter>>, tools: Arc<ToolRegistry>) -> Self {
        Self { llm, tools }
    }

    /// 执行步骤
    pub async fn execute(&self, blackboard: Blackboard) -> Result<Blackboard> {
        let mut bb = blackboard;
        let task_id = bb.task.id;

        // 取出 execution_plan 以释放对 bb 的借用；
        // 模型选择按 tier 单独 clone（ExecutionPlan 含 model_tier: Copy）
        let model_tier = bb
            .execution_plan
            .as_ref()
            .ok_or_else(|| {
                crate::common::error::EflowError::Internal(
                    "Executor called without execution_plan".to_string(),
                )
            })?
            .model_tier;
        let plan = bb.execution_plan.take().ok_or_else(|| {
            crate::common::error::EflowError::Internal(
                "Executor called without execution_plan".to_string(),
            )
        })?;

        for sub_step in &plan.sub_steps {
            // 检查是否需要 LLM 推理
            let result = if sub_step.tool.is_empty() || sub_step.tool == "llm_reasoning" {
                // 纯 LLM 推理步骤
                self.execute_llm_step(model_tier, sub_step, plan.risk_level)
                    .await?
            } else {
                // 工具执行步骤
                self.execute_tool_step(sub_step, task_id).await?
            };

            let record = ActionRecord {
                timestamp: Utc::now(),
                action: sub_step.action.clone(),
                tool: sub_step.tool.clone(),
                success: result.success,
                summary: result.output.chars().take(200).collect(),
            };

            bb = bb.with_action(record);

            if !result.success {
                // 单步失败不中断整个管道，由 Feedbacker 判定
                tracing::warn!("Step action '{}' returned failure", sub_step.action);
            }
        }

        // 把 plan 放回 Blackboard（Feedbacker 可能需要读取 execution_plan）
        bb.execution_plan = Some(plan);

        Ok(bb)
    }

    /// 纯 LLM 推理
    async fn execute_llm_step(
        &self,
        model_tier: ModelTier,
        step: &TaskStep,
        risk: RiskLevel,
    ) -> Result<ActionResult> {
        let mut llm = self.llm.lock().await;

        let messages = vec![Message::user(format!(
            "任务: {}\n\n请完成任务并提供结果。",
            step.action
        ))];

        // 不传工具定义：execute_llm_step 是「纯推理」路径，工具由本步骤 plan 决定
        // （LLM 的工具自选不在 v1.0 范围）
        let request = ChatRequest::new("", messages).with_cache(0);

        // v1.2 D1: 用 helper 替换内联 CacheKey 构造。retry_count 传 None——
        // v1.1 注释说 step.action 在 rework 时已被 subagent 追加建议（subagent.rs:91），
        // key 自动变；同一 logical call 必 cache 命中。
        let key = cache_key_for_step(step, IntentType::Chat, risk, "default", None);

        let response = llm.chat_cached(model_tier, request, &key).await?;

        Ok(ActionResult {
            success: true,
            output: response.content,
            tool_calls: vec![],
            duration_ms: 0,
        })
    }

    /// 工具执行
    async fn execute_tool_step(
        &self,
        step: &TaskStep,
        task_id: uuid::Uuid,
    ) -> Result<ActionResult> {
        let start = std::time::Instant::now();
        let output = self
            .tools
            .execute(&step.tool, step.params.clone(), task_id)
            .await;
        let duration_ms = start.elapsed().as_millis() as u64;

        match output {
            Ok(tool_output) => {
                // 复制 content 以避免「move 后再借用」
                let content = tool_output.content.clone();
                Ok(ActionResult {
                    success: tool_output.success,
                    output: content.clone(),
                    tool_calls: vec![ToolCallSummary {
                        tool_name: step.tool.clone(),
                        success: tool_output.success,
                        duration_ms,
                        summary: content.chars().take(100).collect(),
                    }],
                    duration_ms,
                })
            }
            Err(e) => Ok(ActionResult {
                success: false,
                output: t!("status_action_tool_failed", msg = e.to_string()).to_string(),
                tool_calls: vec![],
                duration_ms,
            }),
        }
    }
}
