//! StandardWorkflow —— 三角色管线，1 次反馈回路（v1.2 Orchestrator 默认行为）
//!
//! 关键设计决策：
//! - 复用 v1.0/v1.1 的 orchestrator.execute —— 零分叉
//! - 1 次反馈回路（feedbacker 在每个 step 内部跑，v1.2 默认）
//!
//! v1.3.3 deviation: Orchestrator.execute 返 String（v1.2 行为）—— WorkflowExecutor
//! 内部包装成 AggregatedResult

use async_trait::async_trait;

use crate::common::error::Result;
use crate::workflow::{AggregatedResult, WorkflowContext, WorkflowExecutor, WorkflowLevel};

pub struct StandardWorkflow;

#[async_trait]
impl WorkflowExecutor for StandardWorkflow {
    fn level(&self) -> WorkflowLevel {
        WorkflowLevel::Standard
    }

    fn description(&self) -> &'static str {
        // v1.3.3 #13i: hard-code（与 simple 同因）
        "标准档：三角色管线（决策+执行+反馈）。适合中等任务"
    }

    async fn execute(&self, ctx: &mut WorkflowContext<'_>) -> Result<AggregatedResult> {
        // v1.3.3 #13a: Orchestrator 是 Arc<Mutex<>> —— Arc clone 出来 lock
        let orch_arc = ctx.orchestrator.clone();
        let summary = {
            let mut orch = orch_arc.lock().await;
            orch.execute(ctx.task.clone()).await?
        };
        Ok(AggregatedResult::new(summary, WorkflowLevel::Standard))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_returns_standard() {
        assert_eq!(StandardWorkflow.level(), WorkflowLevel::Standard);
    }

    #[test]
    fn description_non_empty() {
        assert!(!StandardWorkflow.description().is_empty());
    }
}
