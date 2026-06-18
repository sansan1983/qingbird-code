//! send handler —— 派发 task 给 Concierge
//!
//! 设计：复用 Concierge::handle_input（v1.3.1 已有）—— GUI 在 stdin 发 `send` action
//! 等价于用户在 TUI 输入 task。Concierge 内部生成 task_id 并异步派发到 Orchestrator。
//!
//! `task_id` 字段：GUI 可以传（保持 task_id 跨进程可追踪），也可以不传（Concierge 内部生成）
//! —— 当前 v1.3.2 不做 task_id 注入，保留字段供将来 audit log 关联。
use crate::application::concierge::Concierge;
use crate::common::error::Result;
use uuid::Uuid;

pub async fn dispatch(concierge: &mut Concierge, _task_id: Option<Uuid>, task: &str) -> Result<()> {
    // handle_input 内部走 classify_intent → TaskDispatch 路径，
    // 自动 recall 记忆 + 异步 spawn Orchestrator
    let _ack = concierge.handle_input(task.to_string()).await;
    Ok(())
}
