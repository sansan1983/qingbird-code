//! level handler —— 切换工作流档位
//!
//! 设计：委托到 SlashCommand 链路（`/level` 命令已注册）—— `LevelCmd::parse_args`
//! 校验 ALLOWED_LEVELS = simple/standard/advanced/auto，`execute` 输出 info 消息
//! （v1.3.3 spec C 实施时填具体逻辑）。走 SlashCommand 链路保证契约一致。
use crate::application::concierge::Concierge;
use crate::common::error::Result;
use uuid::Uuid;

pub async fn dispatch(concierge: &mut Concierge, _task_id: Uuid, level: &str) -> Result<()> {
    // 委托 /level 斜杠命令：dispatch_slash 内部 parse_args 校验 + execute 输出
    let _ack = concierge.dispatch_slash(&format!("level {level}")).await;
    Ok(())
}
