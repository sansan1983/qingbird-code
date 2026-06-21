//! step 4: 填 API KEY

use std::io::BufRead;
use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::KeyEvent;
use rust_i18n::t;

use crate::interaction::render::view_model::*;
use crate::interaction::wizard::{StepAction, WizardState, WizardStep};

pub struct ApikeyStep;

fn read_line_from_stdin() -> Option<String> {
    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).ok()?;
    Some(line.trim().to_string())
}

#[async_trait]
impl WizardStep for ApikeyStep {
    fn id(&self) -> &'static str {
        "apikey"
    }
    fn title(&self) -> &'static str {
        "请输入 API KEY"
    }
    fn view_model(&self, state: &WizardState) -> StepViewModel {
        StepViewModel {
            title: t!("wizard_step_title_apikey").to_string(),
            lines: vec![
                LineVM { text: "".into() },
                LineVM {
                    text: "（v1.3.1 阶段：从 stdin 输入 KEY）".into(),
                },
                LineVM { text: "".into() },
                LineVM {
                    text: format!(
                        "API KEY: {}",
                        state.provider_api_key.as_deref().unwrap_or("")
                    ),
                },
            ],
            input: Some(InputFieldVM {
                label: "api_key".into(),
                value: state.provider_api_key.clone().unwrap_or_default(),
                cursor_pos: state
                    .provider_api_key
                    .as_ref()
                    .map(|s| s.len())
                    .unwrap_or(0),
            }),
            hints: vec![
                KeyHint {
                    key: "Enter".into(),
                    description: "下一步".into(),
                },
                KeyHint {
                    key: "Esc".into(),
                    description: "取消".into(),
                },
            ],
            focused_field: 0,
        }
    }
    fn on_key(&self, _key: KeyEvent, state: &mut WizardState) -> StepAction {
        // 简化：直接读 stdin 一行
        let key = match read_line_from_stdin() {
            Some(k) if !k.is_empty() => k,
            _ => return StepAction::Stay,
        };
        state.provider_api_key = Some(key);
        StepAction::Next
    }
    fn is_complete(&self, state: &WizardState) -> bool {
        state.provider_api_key.is_some()
    }
    fn next_step(&self) -> Option<Arc<dyn WizardStep>> {
        Some(Arc::new(super::model::ModelStep))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_complete_false_when_no_key() {
        let step = ApikeyStep;
        let state = WizardState::default();
        assert!(!step.is_complete(&state));
    }

    #[test]
    fn is_complete_true_when_key_set() {
        let step = ApikeyStep;
        let state = WizardState {
            provider_api_key: Some("sk-test".into()),
            ..WizardState::default()
        };
        assert!(step.is_complete(&state));
    }

    #[test]
    fn view_model_has_input_field() {
        let step = ApikeyStep;
        let state = WizardState {
            provider_api_key: Some("sk-abc".into()),
            ..WizardState::default()
        };
        let vm = step.view_model(&state);
        assert!(vm.input.is_some(), "apikey step should have input field");
        let input = vm.input.unwrap();
        assert_eq!(input.value, "sk-abc");
        assert_eq!(input.cursor_pos, 6);
        assert_eq!(input.label, "api_key");
    }

    #[test]
    fn view_model_empty_input_when_no_key() {
        let step = ApikeyStep;
        let state = WizardState::default();
        let vm = step.view_model(&state);
        let input = vm.input.expect("should have input");
        assert!(input.value.is_empty());
        assert_eq!(input.cursor_pos, 0);
    }
}
