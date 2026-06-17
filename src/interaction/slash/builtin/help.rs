//! /help — 列出所有命令

use async_trait::async_trait;

use crate::common::error::Result;
use crate::interaction::slash::{
    CommandContext, CommandRegistry, SlashArgs, SlashCommand, SlashOutput,
};

/// /help 命令——构造时**捕获** registry 的命令列表引用
///
/// 不让 HelpCmd 内部再持 registry 引用（避免循环依赖）。
pub struct HelpCmd {
    commands: Vec<(&'static str, &'static str)>,
}

impl HelpCmd {
    pub fn new(registry: &CommandRegistry) -> Self {
        Self {
            commands: registry.list(),
        }
    }
}

#[async_trait]
impl SlashCommand for HelpCmd {
    fn name(&self) -> &'static str {
        "help"
    }
    fn help(&self) -> &'static str {
        "列出所有命令（输入 /<name> <args> 执行）"
    }
    async fn execute(&self, _args: SlashArgs, _ctx: &mut CommandContext) -> Result<SlashOutput> {
        let mut text = String::from("可用命令:\n");
        for (name, help) in &self.commands {
            text.push_str(&format!("  /{:<12} {}\n", name, help));
        }
        text.push_str("\n运行 `eflow init` 进入配置向导");
        Ok(SlashOutput::Text(text))
    }
}
