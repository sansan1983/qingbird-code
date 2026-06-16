use super::blackboard::Blackboard;
use crate::common::error::Result;
use crate::common::types::{
    ExecutionPlan, IntentType, ModelTier, PlannedStep, RiskLevel, TaskStep,
};
use crate::infrastructure::llm::cache::cache_key_for_step;
use crate::infrastructure::llm::{ChatRequest, LlmRouter, Message};

/// Decisioner — 风险评估 + 执行计划生成 + 模型路由
pub struct Decisioner {
    llm: std::sync::Arc<tokio::sync::Mutex<LlmRouter>>,
}

impl Decisioner {
    pub fn new(llm: std::sync::Arc<tokio::sync::Mutex<LlmRouter>>) -> Self {
        Self { llm }
    }

    /// 评估步骤风险并生成执行计划
    pub async fn decide(&self, blackboard: &Blackboard) -> Result<Blackboard> {
        let step = blackboard
            .current_step
            .as_ref()
            .expect("Decisioner called without current_step");

        // 规则先行：已知风险等级直接路由，不额外调 LLM
        let risk = blackboard.risk_level;
        let model_tier = match risk {
            RiskLevel::L0 => ModelTier::Light,
            RiskLevel::L1 => ModelTier::Light,
            RiskLevel::L2 => ModelTier::Medium,
            RiskLevel::L3 => ModelTier::Strong,
        };

        // 检查是否需要拆分子步骤（仅 L2+ 需要 LLM 规划）
        let sub_steps = if risk >= RiskLevel::L2 {
            self.plan_sub_steps(step, risk, blackboard.retry_count)
                .await?
        } else {
            vec![step.clone()]
        };

        let execution_plan = ExecutionPlan {
            step: PlannedStep {
                order: 0,
                action: step.action.clone(),
                tool: step.tool.clone(),
                params: step.params.clone(),
                depends_on: None,
            },
            model_tier,
            risk_level: risk,
            sub_steps,
        };

        Ok(blackboard.clone().with_execution_plan(execution_plan))
    }

    /// 对复杂步骤调用 LLM 拆分子步骤
    async fn plan_sub_steps(
        &self,
        step: &TaskStep,
        risk: RiskLevel,
        retry_count: u8,
    ) -> Result<Vec<TaskStep>> {
        let mut llm = self.llm.lock().await;

        let messages = vec![
            Message::system("你是一个任务规划专家。将以下操作拆分为更小的子步骤。"),
            Message::user(format!(
                "操作: {}\n工具: {}\n参数: {}\n风险等级: {:?}\n\n请给出拆分后的子步骤列表。",
                step.action, step.tool, step.params, risk
            )),
        ];

        let request = ChatRequest::new("", messages).with_cache(0);

        // v1.2 D1: 用 helper 替换内联 CacheKey 构造。retry_count 仍传 Some——
        // v1.1 注释明示这是为了 break rework loop（同一 step 多次 decide 需不同 plan）。
        let key = cache_key_for_step(step, IntentType::Chat, risk, "default", Some(retry_count));

        let response = llm.chat_cached(ModelTier::Strong, request, &key).await?;

        // 简化解析：每行一个子步骤
        let sub_steps: Vec<TaskStep> = response
            .content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| TaskStep {
                action: line.to_string(),
                tool: step.tool.clone(),
                params: serde_json::json!({}),
                expected_output: None,
            })
            .collect();

        if sub_steps.is_empty() {
            Ok(vec![step.clone()])
        } else {
            Ok(sub_steps)
        }
    }
}
