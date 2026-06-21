//! step 3: 协议选择（仅自定义时显示）

use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use rust_i18n::t;

use crate::infrastructure::llm::types::ProtocolKind;
use crate::interaction::render::view_model::*;
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
    fn view_model(&self, state: &WizardState) -> StepViewModel {
        if state.skip_protocol_step {
            return StepViewModel {
                title: "协议".into(),
                lines: vec![LineVM {
                    text: "(预设厂商已自动选择协议，已跳过)".into(),
                }],
                input: None,
                hints: vec![],
                focused_field: 0,
            };
        }
        StepViewModel {
            title: t!("wizard_step_title_protocol").to_string(),
            lines: vec![
                LineVM { text: "".into() },
                LineVM {
                    text: "  1. openai_compatible (OpenAI 兼容)".into(),
                },
                LineVM {
                    text: "  2. anthropic_compatible (Anthropic 兼容)".into(),
                },
                LineVM { text: "".into() },
                LineVM {
                    text: "输入序号选择 / Esc 取消".into(),
                },
            ],
            input: None,
            hints: vec![
                KeyHint {
                    key: "1/2".into(),
                    description: "选择".into(),
                },
                KeyHint {
                    key: "Esc".into(),
                    description: "取消".into(),
                },
            ],
            focused_field: 0,
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
