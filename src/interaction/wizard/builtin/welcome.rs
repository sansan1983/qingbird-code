//! step 0: 欢迎 + 提示

use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::KeyEvent;

use crate::interaction::render::view_model::*;
use crate::interaction::wizard::{StepAction, WizardState, WizardStep};

pub struct WelcomeStep;

#[async_trait]
impl WizardStep for WelcomeStep {
    fn id(&self) -> &'static str {
        "welcome"
    }
    fn title(&self) -> &'static str {
        "欢迎使用 eflow — 首次配置向导"
    }
    fn view_model(&self, _state: &WizardState) -> StepViewModel {
        StepViewModel {
            title: "欢迎使用 eflow — 首次配置向导".into(),
            lines: vec![
                LineVM { text: "".into() },
                LineVM {
                    text: "欢迎使用 eflow — 首次配置向导".into(),
                },
                LineVM { text: "".into() },
                LineVM {
                    text: "本向导将引导你完成：".into(),
                },
                LineVM {
                    text: "  1. 选择语言（中/英）".into(),
                },
                LineVM {
                    text: "  2. 选择 LLM 厂商（4 ��预置 + 自定义）".into(),
                },
                LineVM {
                    text: "  3. 选择兼容接口（仅自定义时显示）".into(),
                },
                LineVM {
                    text: "  4. 输入 API KEY".into(),
                },
                LineVM {
                    text: "  5. 选择模型（自动拉取 + 手填 fallback）".into(),
                },
                LineVM {
                    text: "  6. 确认配置".into(),
                },
                LineVM { text: "".into() },
                LineVM {
                    text: "按 Enter 开始 / Esc 取消".into(),
                },
            ],
            input: None,
            hints: vec![],
            focused_field: 0,
        }
    }
    fn on_key(&self, key: KeyEvent, _state: &mut WizardState) -> StepAction {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Enter => StepAction::Next,
            KeyCode::Esc => StepAction::Cancel,
            _ => StepAction::Stay,
        }
    }
    fn is_complete(&self, _state: &WizardState) -> bool {
        true
    }
    fn next_step(&self) -> Option<Arc<dyn WizardStep>> {
        Some(Arc::new(super::language::LanguageStep))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn id_and_title_not_empty() {
        let step = WelcomeStep;
        assert_eq!(step.id(), "welcome");
        assert!(!step.title().is_empty());
    }

    #[test]
    fn enter_returns_next_esc_returns_cancel() {
        let step = WelcomeStep;
        let mut state = WizardState::default();
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert!(matches!(step.on_key(enter, &mut state), StepAction::Next));
        assert!(matches!(step.on_key(esc, &mut state), StepAction::Cancel));
    }

    #[test]
    fn view_model_contains_title_and_instructions() {
        let step = WelcomeStep;
        let state = WizardState::default();
        let vm = step.view_model(&state);
        assert!(vm.title.contains("eflow"));
        assert!(vm.lines.len() >= 5, "welcome step should have multiple lines");
        assert!(vm.input.is_none(), "welcome has no input field");
    }

    #[test]
    fn view_model_lines_include_enter_hint() {
        let step = WelcomeStep;
        let state = WizardState::default();
        let vm = step.view_model(&state);
        let text: String = vm.lines.iter().map(|l| l.text.as_str()).collect();
        assert!(text.contains("Enter"), "should mention Enter key");
        assert!(text.contains("Esc"), "should mention Esc key");
    }
}
