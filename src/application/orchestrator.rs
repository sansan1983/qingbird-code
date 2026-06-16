use std::sync::Arc;

use crate::capability::blackboard::Blackboard;
use crate::capability::decisioner::Decisioner;
use crate::capability::executor::Executor;
use crate::capability::feedbacker::Feedbacker;
use crate::capability::pool::SubagentPool;
use crate::capability::subagent::Subagent;
use crate::capability::tools::ToolRegistry;
use crate::common::error::Result;
use crate::common::types::{
    Capability, ModelTier, PlannedStep, RiskLevel, Role, TaskPlan, TaskSpec, TaskStep,
};
use crate::infrastructure::event::{Event, EventChannel};
use crate::infrastructure::llm::{ChatRequest, LlmRouter, Message};
use rust_i18n::t;

/// Orchestrator — 任务分解 + Subagent 调度 + 结果聚合
pub struct Orchestrator {
    llm: Arc<tokio::sync::Mutex<LlmRouter>>,
    tools: Arc<ToolRegistry>,
    events: EventChannel,
    /// Subagent 池（v1.1 M10.5 接入；None 时退化为 v1.0 单 agent 路径）
    pool: Option<Arc<SubagentPool>>,
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
            pool: None,
        }
    }

    /// 用 SubagentPool 构造（v1.1 M10.5 C4 新增）
    pub fn with_pool(
        llm: Arc<tokio::sync::Mutex<LlmRouter>>,
        tools: Arc<ToolRegistry>,
        events: EventChannel,
        pool: Arc<SubagentPool>,
    ) -> Self {
        Self {
            llm,
            tools,
            events,
            pool: Some(pool),
        }
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
                 可用工具:\n{tools_desc}\n\
                 输出格式：每行一个步骤，格式为 '工具名: 操作描述'"
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
        let planned_steps = plan.steps.clone();
        // v1.2 E3: 在 plan move 进 with_plan 之前先算 layers
        // （compute_step_layers 签名是 &TaskPlan，而 with_plan 要 ownership）
        let layers = Self::compute_step_layers(&plan);
        let bb = Blackboard::new(task).with_plan(plan);

        // 2. 构建管线段组件
        let decisioner = Decisioner::new(self.llm.clone());
        let executor = Executor::new(self.llm.clone(), self.tools.clone());
        let feedbacker = Feedbacker::new(self.llm.clone());

        // 3. 构建依赖分层（v1.2: 用于并行派发；v1.1: 统计 + tracing）
        // v1.2 E3: 调用 compute_step_layers 独立方法（行为不变，纯 refactor）
        let max_layer = layers.len().saturating_sub(1);
        tracing::debug!(
            "task {}: {} steps across {} layer(s)",
            task_id,
            planned_steps.len(),
            max_layer + 1
        );

        // 4. 逐步执行（v1.1 串行；v1.2 改用 layer 并行）
        let mut bb = bb;
        let agent = Subagent::new(
            "default".into(),
            Role::Generalist,
            vec![
                Capability::ReadFile,
                Capability::WriteFile,
                Capability::LlmReasoning,
            ],
        );

        for planned_step in &planned_steps {
            let step = TaskStep {
                action: planned_step.action.clone(),
                tool: planned_step.tool.clone(),
                params: planned_step.params.clone(),
                expected_output: None,
            };
            bb = bb.with_step(step);

            tracing::info!(
                "task {}: executing step '{}' (tool={})",
                task_id,
                planned_step.action,
                planned_step.tool
            );

            // v1.1 池路径：dispatch + take_handle 拿 agent 句柄（drop 即归还）。
            // SubagentHandle 不暴露 Subagent 引用，v1.2 重构后才走纯 pool 执行。
            if let Some(pool) = &self.pool
                && let Ok(id) = pool.dispatch_for_role(Role::Generalist).await
            {
                let _h = pool.take_handle(id);
            }

            bb = agent
                .execute_step(bb, &decisioner, &executor, &feedbacker)
                .await?;
        }

        // 5. 聚合结果
        let summary = bb.summarize();

        self.events.publish(Event::TaskCompleted {
            task_id,
            summary: summary.clone(),
        });

        Ok(summary)
    }

    /// v1.2 E3: 把 TaskPlan 按依赖分层，每层内的步骤可并行执行。
    ///
    /// 算法：广度优先遍历 depends_on 图。
    /// - 无依赖（depends_on=None）→ layer 0
    /// - 依赖的步骤在 layer N → 本步骤在 layer N+1
    /// - 多依赖取最大 layer + 1（plan 步骤按 order 升序遍历，前序步骤 layer 已算）
    ///
    /// 返回 Vec<Vec<u8>>，外层下标 = layer 索引，内层 = 该层步骤的 order 列表
    ///
    /// 关联：v1.2 E4 按层 FuturesUnordered 并行派发；E3 抽方法，E4 用结果
    ///
    /// 可见性：v1.2 plan §E3 step 3 写 `pub(crate)`，但 tests/ 集成测试是独立
    /// crate 看不到 pub(crate) —— 改为 `pub` 让 integration test 可见。
    #[must_use]
    pub fn compute_step_layers(plan: &TaskPlan) -> Vec<Vec<u8>> {
        let mut step_to_layer: std::collections::HashMap<u8, usize> =
            std::collections::HashMap::new();
        let mut layers: Vec<Vec<u8>> = vec![vec![]];

        for step in &plan.steps {
            let layer = match step.depends_on {
                None => 0,
                Some(dep) => step_to_layer.get(&dep).copied().unwrap_or(0) + 1,
            };
            while layers.len() <= layer {
                layers.push(vec![]);
            }
            layers[layer].push(step.order);
            step_to_layer.insert(step.order, layer);
        }
        layers
    }
}
