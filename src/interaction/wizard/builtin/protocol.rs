//! step 3: 协议选择（仅自定义时显示）

use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::prelude::{Buffer, Widget};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use rust_i18n::t;

use crate::infrastructure::llm::types::ProtocolKind;
use crate::interaction::wizard::{StepAction, WizardState, WizardStep};

pub struct ProtocolStep;

#[async_trait]
impl WizardStep for ProtocolStep {
    fn id(&self) -> &'static str {
        "protocol"
    }
    fn title(&self) -> &'static str {
        "选择兼容接口协议"
    }
    fn render(&self, area: Rect, buf: &mut Buffer, state: &WizardState) {
        // 临时硬编码
        if state.skip_protocol_step {
            let text = vec![Line::from("(预设厂商已自动选择协议，已跳过)")];
            Paragraph::new(text).render(area, buf);
        } else {
            let text = vec![
                Line::from(t!("wizard_step_title_protocol").to_string()),
                Line::from(""),
                Line::from("  1. openai_compatible (OpenAI 兼容)".to_string()),
                Line::from("  2. anthropic_compatible (Anthropic 兼容)".to_string()),
                Line::from(""),
                Line::from("输入序号选择 / Esc 取消"),
            ];
            Paragraph::new(text).render(area, buf);
        }
    }
    fn on_key(&self, key: KeyEvent, state: &mut WizardState) -> StepAction {
        if state.skip_protocol_step {
            return StepAction::Next;
        }
        let protocol = match key.code {
            KeyCode::Char('1') => ProtocolKind::OpenaiCompatible,
            KeyCode::Char('2') => ProtocolKind::AnthropicCompatible,
            KeyCode::Esc => return StepAction::Cancel,
            _ => return StepAction::Stay,
        };
        state.provider_protocol = Some(protocol);
        StepAction::Next
    }
    fn is_complete(&self, _state: &WizardState) -> bool {
        true
    }
    fn next_step(&self) -> Option<Arc<dyn WizardStep>> {
        Some(Arc::new(super::apikey::ApikeyStep))
    }
}
