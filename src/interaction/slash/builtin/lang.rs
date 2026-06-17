//! /lang <locale> — 切换语言

use async_trait::async_trait;
use rust_i18n::t;

use crate::common::error::{EflowError, Result};
use crate::infrastructure::locale;
use crate::interaction::slash::{CommandContext, SlashArgs, SlashCommand, SlashOutput};

pub struct LangCmd;

#[async_trait]
impl SlashCommand for LangCmd {
    fn name(&self) -> &'static str {
        "lang"
    }
    fn help(&self) -> &'static str {
        "切换语言（zh-CN / en-US）"
    }
    fn parse_args(&self, raw: &str) -> Result<SlashArgs> {
        let lang = raw.trim();
        if !locale::SUPPORTED_LOCALES.contains(&lang) {
            return Err(EflowError::Config(
                t!("err_invalid_lang", lang = lang).into_owned(),
            ));
        }
        Ok(SlashArgs::from_kv(&[("arg0", lang)]))
    }
    async fn execute(&self, args: SlashArgs, _ctx: &mut CommandContext) -> Result<SlashOutput> {
        let lang = args.first().cloned().unwrap_or_default();
        locale::init(Some(&lang));
        Ok(SlashOutput::Text(
            t!("status_lang_changed", lang = lang).into_owned(),
        ))
    }
}
