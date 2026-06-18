//! /level <simple|standard|advanced|auto> — 切换工作流档位
//!
//! **v1.3.3 阶段填具体逻辑**——通过 `ctx.concierge.workflow_registry_mut()`
//! 调 set_override。auto 清除 override，回到 Concierge::determine_workflow_level
//! 自动判定。`parse_args` 已经在 v1.3.1 校验 4 档（simple/standard/advanced/auto），
//! execute match 不会失败（_ 分支 unreachable!()）。

use async_trait::async_trait;
use rust_i18n::t;

use crate::common::error::{EflowError, Result};
use crate::interaction::slash::{CommandContext, SlashArgs, SlashCommand, SlashOutput};
use crate::workflow::WorkflowLevel;

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
    async fn execute(&self, args: SlashArgs, ctx: &mut CommandContext) -> Result<SlashOutput> {
        let level = args.first().cloned().unwrap_or_default();
        // v1.3.3 增量：set_override(None) = /level auto，回自动判定
        match level.as_str() {
            "auto" => ctx.concierge.workflow_registry_mut().set_override(None),
            "simple" => ctx
                .concierge
                .workflow_registry_mut()
                .set_override(Some(WorkflowLevel::Simple)),
            "standard" => ctx
                .concierge
                .workflow_registry_mut()
                .set_override(Some(WorkflowLevel::Standard)),
            "advanced" => ctx
                .concierge
                .workflow_registry_mut()
                .set_override(Some(WorkflowLevel::Advanced)),
            // parse_args 已校验 _ 不会出现
            _ => unreachable!("parse_args validates this"),
        }
        Ok(SlashOutput::Text(
            t!("status_level_changed", level = level).into_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_validates_4_levels() {
        let cmd = LevelCmd;
        assert!(cmd.parse_args("simple").is_ok());
        assert!(cmd.parse_args("standard").is_ok());
        assert!(cmd.parse_args("advanced").is_ok());
        assert!(cmd.parse_args("auto").is_ok());
        assert!(cmd.parse_args("turbo").is_err());
    }
}
