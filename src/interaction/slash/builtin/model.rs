//! /model — 弹 SelectList 子视图切模型
//!
//! v1.3.1 阶段：返回 OpenSubView 让 TUI 主循环切换到 SelectList 子视图。
//! v1.3.2 spec B 实施时 GUI 也复用此 trait impl（跨进程时通过 stdin send 触发）。

use std::sync::Arc;

use async_trait::async_trait;

use crate::common::error::{EflowError, Result};
use crate::common::types::ModelTier;
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
        // v1.3.1 阶段：检查 router 是否有 provider，没有 → 错误
        let router = ctx.router.lock().await;
        if router.provider_for(ModelTier::Light).is_none() {
            return Err(EflowError::Config(
                "无法切换模型：未配置 LLM provider。运行 qingbird init 配置".into(),
            ));
        }

        // 拿当前 provider id（这里简化：直接拿 Light tier 的 provider id）
        let provider_id = router.provider_for(ModelTier::Light).unwrap().to_string();

        // 简化：从 router 拿当前 provider 的 preset_models（v1.3.1 阶段没有 model cache 抽象）
        // v1.3.1 计划走"返回 None"——`/model` 显示"暂无模型列表"
        let preset_models = router.preset_models_for(&provider_id).unwrap_or_default();
        drop(router); // 提前释放锁——下面 SelectList::load 不需要 router

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
