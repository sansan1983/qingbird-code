//! /quit — 退出 eflow

use async_trait::async_trait;

use crate::common::error::Result;
use crate::interaction::slash::{CommandContext, SlashArgs, SlashCommand, SlashOutput};

pub struct QuitCmd;

#[async_trait]
impl SlashCommand for QuitCmd {
    fn name(&self) -> &'static str {
        "quit"
    }
    fn help(&self) -> &'static str {
        "退出 eflow"
    }
    async fn execute(&self, _args: SlashArgs, _ctx: &mut CommandContext) -> Result<SlashOutput> {
        Ok(SlashOutput::Shutdown)
    }
}
