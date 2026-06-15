use std::sync::Arc;
use chrono::Utc;

use crate::common::error::Result;
use crate::common::types::*;
use crate::infrastructure::llm::{ChatRequest, LlmRouter, Message};
use crate::capability::tools::ToolRegistry;
use super::blackboard::Blackboard;
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
        let plan = blackboard
            .execution_plan
            .as_ref()
            .expect("Executor called without execution_plan");

        let mut bb = blackboard;
        let start = std::time::Instant::now();

        for sub_step in &plan.sub_steps {
            // 检查是否需要 LLM 推理
            let result = if sub_step.tool.is_empty() || sub_step.tool == "llm_reasoning" {
                // 纯 LLM 推理步骤
                self.execute_llm_step(plan, sub_step).await?
            } else {
                // 工具执行步骤
                self.execute_tool_step(sub_step).await?
            };

            let duration_ms = start.elapsed().as_millis() as u64;

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

        Ok(bb)
    }

    /// 纯 LLM 推理
    async fn execute_llm_step(
        &self,
        plan: &ExecutionPlan,
        step: &TaskStep,
    ) -> Result<ActionResult> {
        let mut llm = self.llm.lock().await;

        let messages = vec![Message::user(format!(
            "任务: {}\n\n请完成任务并提供结果。",
            step.action
        ))];

        // 不传工具定义：execute_llm_step 是「纯推理」路径，工具由本步骤 plan 决定
        // （LLM 的工具自选不在 v1.0 范围）
        let request = ChatRequest::new("", messages).with_cache(0);

        let response = llm.chat(plan.model_tier, request).await?;

        Ok(ActionResult {
            success: true,
            output: response.content,
            tool_calls: vec![],
            duration_ms: 0,
        })
    }

    /// 工具执行
    async fn execute_tool_step(&self, step: &TaskStep) -> Result<ActionResult> {
        let start = std::time::Instant::now();
        let output = self.tools.execute(&step.tool, step.params.clone()).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        match output {
            Ok(tool_output) => Ok(ActionResult {
                success: tool_output.success,
                output: tool_output.content,
                tool_calls: vec![ToolCallSummary {
                    tool_name: step.tool.clone(),
                    success: tool_output.success,
                    duration_ms,
                    summary: tool_output.content.chars().take(100).collect(),
                }],
                duration_ms,
            }),
            Err(e) => Ok(ActionResult {
                success: false,
                output: t!("status_action_tool_failed", msg = e.to_string()),
                tool_calls: vec![],
                duration_ms,
            }),
        }
    }
}
