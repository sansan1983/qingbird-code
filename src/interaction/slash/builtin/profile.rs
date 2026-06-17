//! /profile <name> — 切换行业 profile

use async_trait::async_trait;

use crate::common::error::{EflowError, Result};
use crate::interaction::slash::{CommandContext, SlashArgs, SlashCommand, SlashOutput};

pub struct ProfileCmd;

#[async_trait]
impl SlashCommand for ProfileCmd {
    fn name(&self) -> &'static str {
        "profile"
    }
    fn help(&self) -> &'static str {
        "切换行业 profile（<name>）"
    }
    fn parse_args(&self, raw: &str) -> Result<SlashArgs> {
        let name = raw.trim();
        if name.is_empty() {
            return Err(EflowError::Config("profile 命令需要 <name> 参数".into()));
        }
        Ok(SlashArgs::from_kv(&[("arg0", name)]))
    }
    async fn execute(&self, args: SlashArgs, ctx: &mut CommandContext) -> Result<SlashOutput> {
        let name = args
            .first()
            .ok_or_else(|| EflowError::Config("profile 命令需要 <name> 参数".into()))?;
        ctx.concierge.set_active_profile(name.clone()).await;
        Ok(SlashOutput::Text(format!("已切换 profile 到 {}", name)))
    }
}
