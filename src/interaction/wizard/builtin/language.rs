//! step 1: 语言选择

use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use rust_i18n::t;

use crate::infrastructure::locale;
use crate::interaction::render::view_model::*;
use crate::interaction::wizard::{StepAction, WizardState, WizardStep};

pub struct LanguageStep;

#[async_trait]
impl WizardStep for LanguageStep {
    fn id(&self) -> &'static str {
        "language"
    }
    fn title(&self) -> &'static str {
        "选择语言"
    }
    fn view_model(&self, _state: &WizardState) -> StepViewModel {
        StepViewModel {
            title: t!("wizard_step_title_language").to_string(),
            lines: vec![
                LineVM { text: "".into() },
                LineVM {
                    text: "  1. zh-CN (默认)".into(),
                },
                LineVM {
                    text: "  2. en-US".into(),
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
                    description: "切换".into(),
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
        let lang = match key.code {
            KeyCode::Char('1') => "zh-CN",
            KeyCode::Char('2') => "en-US",
            KeyCode::Esc => return StepAction::Cancel,
            _ => return StepAction::Stay,
        };
        state.config.core.language = lang.into();
        locale::init(Some(lang));
        StepAction::Next
    }
    fn is_complete(&self, _state: &WizardState) -> bool {
        true
    }
    fn next_step(&self) -> Option<Arc<dyn WizardStep>> {
        Some(Arc::new(super::provider::ProviderStep))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn char_1_sets_zh_cn() {
        let step = LanguageStep;
        let mut state = WizardState::default();
        let key = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
        step.on_key(key, &mut state);
        assert_eq!(state.config.core.language, "zh-CN");
    }

    #[test]
    fn char_2_sets_en_us() {
        let step = LanguageStep;
        let mut state = WizardState::default();
        let key = KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE);
        let action = step.on_key(key, &mut state);
        assert!(matches!(action, StepAction::Next));
        assert_eq!(state.config.core.language, "en-US");
    }
}
