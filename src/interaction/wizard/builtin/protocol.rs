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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::llm::types::ProtocolKind;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn skip_protocol_step_returns_next_immediately() {
        let step = ProtocolStep;
        let mut state = WizardState {
            skip_protocol_step: true,
            ..WizardState::default()
        };
        // 任何 key 都应返回 Next
        let key = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
        let action = step.on_key(key, &mut state);
        assert!(matches!(action, StepAction::Next));
    }

    #[test]
    fn char_1_sets_openai_compatible() {
        let step = ProtocolStep;
        let mut state = WizardState::default();
        let key = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
        let action = step.on_key(key, &mut state);
        assert!(matches!(action, StepAction::Next));
        assert!(matches!(
            state.provider_protocol,
            Some(ProtocolKind::OpenaiCompatible)
        ));
    }
}
