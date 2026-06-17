//! /level <simple|standard|advanced|auto> — 切换工作流档位
//!
//! **v1.3.1 阶段只挂空壳**——校验参数 + 输出 info message。
//! v1.3.2 spec C 实施时填具体逻辑（调 WorkflowExecutor trait）。

use async_trait::async_trait;
use rust_i18n::t;

use crate::common::error::{EflowError, Result};
use crate::interaction::slash::{CommandContext, SlashArgs, SlashCommand, SlashOutput};

pub struct LevelCmd;

const ALLOWED_LEVELS: &[&str] = &["simple", "standard", "advanced", "auto"];

#[async_trait]
impl SlashCommand for LevelCmd {
    fn name(&self) -> &'static str {
        "level"
    }
    fn help(&self) -> &'static str {
        "切换工作流档位（simple/standard/advanced/auto）"
    }
    fn parse_args(&self, raw: &str) -> Result<SlashArgs> {
        let level = raw.trim();
        if !ALLOWED_LEVELS.contains(&level) {
            return Err(EflowError::Config(
                t!("err_invalid_level", level = level).into_owned(),
            ));
        }
        Ok(SlashArgs::from_kv(&[("arg0", level)]))
    }
    async fn execute(&self, args: SlashArgs, _ctx: &mut CommandContext) -> Result<SlashOutput> {
        let level = args.first().cloned().unwrap_or_default();
        // v1.3.1 阶段只输出 info message
        // v1.3.2 spec C 实施时填：调 WorkflowExecutor trait + set_override
        Ok(SlashOutput::Text(
            t!("info_level_pending_spec_c", level = level).into_owned(),
        ))
    }
}
