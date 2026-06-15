use std::sync::Arc;

use crate::capability::blackboard::Blackboard;
use crate::capability::decisioner::Decisioner;
use crate::capability::executor::Executor;
use crate::capability::feedbacker::Feedbacker;
use crate::capability::subagent::Subagent;
use crate::capability::tools::ToolRegistry;
use crate::common::error::Result;
use crate::common::types::*;
use crate::infrastructure::event::{Event, EventChannel};
use crate::infrastructure::llm::{ChatRequest, LlmRouter, Message};
use rust_i18n::t;

/// Orchestrator — 任务分解 + Subagent 调度 + 结果聚合
pub struct Orchestrator {
    llm: Arc<tokio::sync::Mutex<LlmRouter>>,
    tools: Arc<ToolRegistry>,
    events: EventChannel,
    /// 当前活跃的 Subagent（test-visible，v1.1 C4 改为 SubagentPool 时整体删除）
    pub active_agent: Option<Subagent>,
}

impl Orchestrator {
    pub fn new(
        llm: Arc<tokio::sync::Mutex<LlmRouter>>,
        tools: Arc<ToolRegistry>,
        events: EventChannel,
    ) -> Self {
        Self {
            llm,
            tools,
            events,
            active_agent: None,
        }
    }

    /// 确保有一个可用的 Subagent（test-visible，v1.1 C4 改为 SubagentPool 时整体删除）
    pub fn ensure_agent(&mut self) -> &Subagent {
        if self.active_agent.is_none() {
            self.active_agent = Some(Subagent::new(
                "default".into(),
                Role::Generalist,
                vec![
                    Capability::ReadFile,
                    Capability::WriteFile,
                    Capability::LlmReasoning,
                ],
            ));
        }
        self.active_agent.as_ref().unwrap()
    }

    /// LLM 驱动的任务分解
    pub async fn decompose(&self, task: &TaskSpec) -> Result<TaskPlan> {
        let mut llm = self.llm.lock().await;

        // 简单任务：规则分解
        if task.risk_level <= RiskLevel::L1 && task.description.len() < 100 {
            return Ok(TaskPlan {
                task_id: task.id,
                steps: vec![PlannedStep {
                    order: 0,
                    action: task.description.clone(),
                    tool: "llm_reasoning".into(),
                    params: serde_json::json!({}),
                    depends_on: None,
                }],
                estimated_steps: 1,
                risk_level: task.risk_level,
            });
        }

        // 复杂任务：LLM 分解
        let tool_defs = self.tools.definitions();
        let tools_desc: String = tool_defs
            .iter()
            .map(|t| format!("- {}: {}", t.name, t.description))
            .collect();

        let messages = vec![
            Message::system(format!(
                "你是一个任务规划专家。将用户任务分解为可执行的步骤序列。\n\
                 可用工具:\n{}\n\
                 输出格式：每行一个步骤，格式为 '工具名: 操作描述'",
                tools_desc
            )),
            Message::user(format!("请分解以下任务:\n{}", task.description)),
        ];

        let request = ChatRequest::new("", messages);
        let response = llm.chat(ModelTier::Strong, request).await?;

        let default_action = t!("status_orchestrator_default_action").to_string();
        let steps: Vec<PlannedStep> = response
            .content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .enumerate()
            .map(|(i, line)| {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                let tool = parts
                    .first()
                    .copied()
                    .unwrap_or("llm_reasoning")
                    .trim()
                    .to_string();
                let action = parts
                    .get(1)
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| default_action.clone());

                PlannedStep {
                    order: i as u8,
                    action,
                    tool,
                    params: serde_json::json!({}),
                    depends_on: if i > 0 { Some(i as u8 - 1) } else { None },
                }
            })
            .collect();

        let estimated_steps = steps.len() as u8;
        Ok(TaskPlan {
            task_id: task.id,
            steps,
            estimated_steps,
            risk_level: task.risk_level,
        })
    }

    /// 执行任务：TaskSpec → 分解 → 管线段执行 → 聚合结果
    pub async fn execute(&mut self, task: TaskSpec) -> Result<String> {
        let task_id = task.id;
        self.events.publish(Event::TaskStarted {
            task_id,
            description: task.description.clone(),
        });

        // 1. 分解任务为步骤
        let plan = self.decompose(&task).await?;
        // 克隆 steps 避免循环迭代器与后续 bb 移动的借用冲突
        let planned_steps = plan.steps.clone();
        let bb = Blackboard::new(task).with_plan(plan);

        // 2. 构建管线段组件
        let decisioner = Decisioner::new(self.llm.clone());
        let executor = Executor::new(self.llm.clone(), self.tools.clone());
        let feedbacker = Feedbacker::new(self.llm.clone());

        // 3. 逐步执行
        let agent = self.ensure_agent();
        let mut bb = bb;

        for planned_step in planned_steps.iter() {
            let step = TaskStep {
                action: planned_step.action.clone(),
                tool: planned_step.tool.clone(),
                params: planned_step.params.clone(),
                expected_output: None,
            };
            bb = bb.with_step(step);

            // 步骤进度走 tracing，不发事件（spec 10.1 事件词汇里没有 StepStarted）
            tracing::info!(
                "task {}: executing step '{}' (tool={})",
                task_id,
                planned_step.action,
                planned_step.tool
            );

            bb = agent
                .execute_step(bb, &decisioner, &executor, &feedbacker)
                .await?;
        }

        // 4. 聚合结果
        let summary = bb.summarize();

        self.events.publish(Event::TaskCompleted {
            task_id,
            summary: summary.clone(),
        });

        Ok(summary)
    }
}
