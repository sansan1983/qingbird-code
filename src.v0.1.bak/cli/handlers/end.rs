//! end handler —— 取消 task 并退出会话
//!
//! 设计：end 触发 read_loop 退出（返 0），handler 本体只 stderr 输出取消信息。
//! `task_id` 字段保留供将来 audit 关联；当前不取消已派发 task（spec B2 §3.6 简化为「end = 退出会话」）
use crate::application::concierge::Concierge;
use crate::cli::output::CliOutput;
use crate::common::error::Result;
use uuid::Uuid;

pub async fn dispatch(_concierge: &mut Concierge, task_id: Uuid) -> Result<()> {
    CliOutput::info(&format!("end task {task_id}: 准备退出会话"));
    Ok(())
}
