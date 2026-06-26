//! /level <simple|standard|advanced|auto> — 切换工作流档位
//!
//! **v1.3.3+ 阶段占位** —— v1.3.3 spec C 实施未接通派发路径（PR 收尾时
//! 删整套 workflow 抽象），/level 命令暂时只 echo 解析结果。v1.4+ 实施
//! 多档位语义时此处重写为真 set_override 链路。
//!
//! `parse_args` 保留 4 档（simple/standard/advanced/auto）校验——历史
//! 兼容 + 后续真接线时不需要改 help/parse 路径。

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
        "切换工作流档位（simple/standard/advanced/auto）—— v1.3.3 占位"
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
        // v1.3.3 收尾：workflow 抽象已删，此处只 echo 解析结果
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
