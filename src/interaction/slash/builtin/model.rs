//! /model — 弹 SelectList 子视图切模型
//!
//! v1.3.1 阶段：返回 OpenSubView 让 TUI 主循环切换到 SelectList 子视图。
//! v1.3.2 spec B 实施时 GUI 也复用此 trait impl（跨进程时通过 stdin send 触发）。

use std::sync::Arc;

use async_trait::async_trait;

use crate::common::error::Result;
use crate::interaction::slash::{CommandContext, SlashArgs, SlashCommand, SlashOutput};
use crate::interaction::widgets::select_list::{SelectItem, SelectItemSource, SelectList};

/// /model 命令
///
/// 数据源：当前 provider 的 model 列表（preset_models + 拉取缓存，v1.3.1 阶段用 preset_models 兜底）
pub struct ModelCmd;

/// ModelListSource 从 LlmRouter 当前 provider 拿 preset_models
pub struct ModelListSource {
    pub provider_id: String,
    pub preset_models: Vec<String>,
}

#[async_trait]
impl SelectItemSource for ModelListSource {
    async fn items(&self) -> Result<Vec<SelectItem>> {
        Ok(self
            .preset_models
            .iter()
            .map(|m| SelectItem::new(m.clone(), m.clone()))
            .collect())
    }
}

#[async_trait]
impl SlashCommand for ModelCmd {
    fn name(&self) -> &'static str {
        "model"
    }
    fn help(&self) -> &'static str {
        "切换当前 provider 的模型（弹 SelectList 子视图）"
    }
    async fn execute(&self, _args: SlashArgs, ctx: &mut CommandContext) -> Result<SlashOutput> {
        // V0.1.0: deepseek 单 provider，provider_name 恒为 "deepseek"
        let router = ctx.router.lock().await;
        let provider_id = router.provider_name().to_string();
        drop(router); // 提前释放锁——下面 SelectList::load 不需要 router

        // V0.1.0: 没有 preset_models 支持，返回空列表
        let preset_models: Vec<String> = Vec::new();

        let source = Arc::new(ModelListSource {
            provider_id,
            preset_models,
        });
        let mut list = SelectList::new(source);
        list.load().await?;
        Ok(SlashOutput::OpenSubView(Arc::new(list)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_name_and_help() {
        // v1.3.1 简化：execute 需要 Concierge placeholder（复杂），T9 集成测试覆盖
        let cmd = ModelCmd;
        assert_eq!(cmd.name(), "model");
        assert!(!cmd.help().is_empty());
    }
}
