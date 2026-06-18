//! AdvancedWorkflow —— 三角色 + 记忆检索 + 包装
//!
//! 关键设计决策：
//! - 记忆检索前置 —— 调 memory.recall，按 description 关键词查 Project scope
//! - v1.2 Orchestrator.execute 跑 1 次反馈（v1.2 默认行为）
//! - summary 包装 —— 标记记忆数量 + 触发源（便于 audit 调试）
//!
//! v1.3.3 deviation #13h: 不真做"3 次反馈"——v1.2 Orchestrator.execute 是 1 次
//! 反馈默认 + Blackboard 不暴露外部 access。**实质"3 次反馈"行为差异留
//! v1.4+ 完善**。Advanced = execute + 记忆检索 + summary 包装，行为差异通过
//! memory.count 标识，**不**改变 Orchestrator 内部管线。

use async_trait::async_trait;

use crate::common::error::Result;
use crate::workflow::{AggregatedResult, WorkflowContext, WorkflowExecutor, WorkflowLevel};

pub struct AdvancedWorkflow;

#[async_trait]
impl WorkflowExecutor for AdvancedWorkflow {
    fn level(&self) -> WorkflowLevel {
        WorkflowLevel::Advanced
    }

    fn description(&self) -> &'static str {
        // v1.3.3 #13i: hard-code（与 simple 同因）
        "高级档：标准 + 记忆检索。适合复杂/重构/系统级任务"
    }

    fn max_retries(&self) -> u8 {
        3 // 覆盖默认
    }

    async fn execute(&self, ctx: &mut WorkflowContext<'_>) -> Result<AggregatedResult> {
        // 1. 记忆检索（按 task 关键词）—— v1.3.3 #13j: CompositeMemory 不
        // implement MemoryManager trait，用 recall_smart(query, limit) 方法。
        // 失败回退到 0（不阻塞）
        let mem_count = {
            let mem_arc = ctx.memory.clone();
            let mem = mem_arc.lock().await;
            mem.recall_smart(&ctx.task.description, 5)
                .map(|v| v.len())
                .unwrap_or_else(|e| {
                    tracing::warn!("Advanced 档记忆检索失败: {e}，回退到 0 条");
                    0
                })
        };

        // 2. 调 orchestrator (v1.2 1 次反馈默认)
        let summary = {
            let orch_arc = ctx.orchestrator.clone();
            let mut orch = orch_arc.lock().await;
            orch.execute(ctx.task.clone()).await?
        };

        // 3. 包装 summary（标 memory count + level）
        let wrapped = format!("[Advanced: recalled {mem_count} memory entries]\n{summary}");
        Ok(AggregatedResult::new(wrapped, WorkflowLevel::Advanced))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_returns_advanced() {
        assert_eq!(AdvancedWorkflow.level(), WorkflowLevel::Advanced);
    }

    #[test]
    fn description_non_empty() {
        assert!(!AdvancedWorkflow.description().is_empty());
    }

    #[test]
    fn max_retries_overrides_default_to_3() {
        assert_eq!(AdvancedWorkflow.max_retries(), 3);
    }
}
