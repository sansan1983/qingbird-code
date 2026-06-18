//! SimpleWorkflow —— 1 次 LLM 直接答，无验证
//!
//! 关键设计决策：
//! - **不走三角色管线**——没有 Decisioner 评估，没有 Executor 工具调用，没有 Feedbacker 验证
//! - **强制 light tier**——Simple 不应该用大模型
//! - **无反馈回路**——错了用户自己改
//!
//! v1.3.3 deviation #13e: Concierge 没 `router_mut()`——SimpleWorkflow 通过
//! `ctx.concierge.llm_router.clone()` 拿 Arc 出来 lock。

use async_trait::async_trait;

use crate::common::error::Result;
use crate::common::types::{ModelTier, TaskSpec};
use crate::infrastructure::llm::types::{ChatRequest, ChatResponse, Message, MessageRole};
use crate::workflow::{AggregatedResult, WorkflowContext, WorkflowExecutor, WorkflowLevel};

pub struct SimpleWorkflow;

#[async_trait]
impl WorkflowExecutor for SimpleWorkflow {
    fn level(&self) -> WorkflowLevel {
        WorkflowLevel::Simple
    }

    fn description(&self) -> &'static str {
        // v1.3.3 #13i: hard-code 字面量——description trait 返 &'static str
        // 而 t!().as_str() 不稳定。locale key workflow_simple_desc 仍加 locales/
        // 备 v1.4+ wired
        "简易档：1 次 LLM 直接答，无验证（查询/小修改）"
    }

    async fn execute(&self, ctx: &mut WorkflowContext<'_>) -> Result<AggregatedResult> {
        // 1. 构造 chat request（用 task description 作为 user message）
        let request = ChatRequest {
            model: String::new(), // 路由层会按 tier 选 model
            messages: vec![Message {
                role: MessageRole::User,
                content: ctx.task.description.clone(),
            }],
            tools: None,
            temperature: 0.7,
            max_tokens: 4096,
            cache_control: None,
        };

        // 2. 1 次 LLM 调用（强制 light tier）—— v1.3.3 #13e: Arc clone 出来 lock
        let router_arc = ctx.concierge.llm_router_handle();
        let response: ChatResponse = {
            let mut router = router_arc.lock().await;
            router.chat(ModelTier::Light, request).await?
        };

        // 3. 返回 AggregatedResult
        Ok(AggregatedResult::new(
            response.content,
            WorkflowLevel::Simple,
        ))
    }
}

#[allow(dead_code)]
fn _ensure_types(_r: &ChatResponse, _t: &TaskSpec) {}
