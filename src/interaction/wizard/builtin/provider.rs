//! step 2: 厂商选择（4 个 preset + 自定义）

use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::prelude::{Buffer, Widget};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use rust_i18n::t;

use crate::interaction::wizard::{StepAction, WizardState, WizardStep};

pub struct ProviderStep;

/// preset YAML 文件来源（4 个 + 自定义 = 5 选项）
fn list_providers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("deepseek", "DeepSeek"),
        ("minimax", "MiniMax"),
        ("agnes-ai", "Agnes AI"),
        ("opencode-go", "OpenCode Go"),
        ("custom", "自定义（兼容 OpenAI / Anthropic）"),
    ]
}

#[async_trait]
impl WizardStep for ProviderStep {
    fn id(&self) -> &'static str {
        "provider"
    }
    fn title(&self) -> &'static str {
        "选择 LLM 厂商"
    }
    fn render(&self, area: Rect, buf: &mut Buffer, _state: &WizardState) {
        // 临时硬编码
        let mut text = vec![
            Line::from(t!("wizard_step_title_provider").to_string()),
            Line::from(""),
        ];
        for (i, (id, name)) in list_providers().iter().enumerate() {
            let hint = if *id == "custom" {
                t!("wizard_provider_hint_custom").to_string()
            } else {
                t!("wizard_provider_hint_preset").to_string()
            };
            text.push(Line::from(format!("  {}. {} {}", i + 1, name, hint)));
        }
        text.push(Line::from(""));
        text.push(Line::from("输入序号选择 / Esc 取消"));
        Paragraph::new(text).render(area, buf);
    }
    fn on_key(&self, key: KeyEvent, state: &mut WizardState) -> StepAction {
        let n = match key.code {
            KeyCode::Char('1') => 1,
            KeyCode::Char('2') => 2,
            KeyCode::Char('3') => 3,
            KeyCode::Char('4') => 4,
            KeyCode::Char('5') => 5,
            KeyCode::Esc => return StepAction::Cancel,
            _ => return StepAction::Stay,
        };
        let (id, name) = list_providers()[n - 1];
        if id == "custom" {
            // 自定义路径：标记需要 protocol 步
            state.skip_protocol_step = false;
            state.provider_id = Some("custom".into());
            state.provider_display_name = Some("Custom".into());
        } else {
            // preset 路径：跳过 protocol 步
            state.skip_protocol_step = true;
            state.provider_id = Some(id.into());
            state.provider_display_name = Some(name.into());
        }
        StepAction::Next
    }
    fn is_complete(&self, _state: &WizardState) -> bool {
        true
    }
    fn next_step(&self) -> Option<Arc<dyn WizardStep>> {
        Some(Arc::new(super::protocol::ProtocolStep))
    }
}
