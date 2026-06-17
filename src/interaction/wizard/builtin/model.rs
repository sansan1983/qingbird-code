//! step 5: 模型选择（拉取 + 手填 fallback）

use std::io::BufRead;
use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::prelude::{Buffer, Widget};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use rust_i18n::t;

use crate::interaction::wizard::{StepAction, WizardState, WizardStep};

pub struct ModelStep;

fn read_line_from_stdin() -> Option<String> {
    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).ok()?;
    Some(line.trim().to_string())
}

#[async_trait]
impl WizardStep for ModelStep {
    fn id(&self) -> &'static str {
        "model"
    }
    fn title(&self) -> &'static str {
        "选择模型"
    }
    fn render(&self, area: Rect, buf: &mut Buffer, state: &WizardState) {
        // 临时硬编码
        let mut text = vec![
            Line::from(if state.fetch_failed {
                t!("wizard_step_title_model_fallback").to_string()
            } else {
                t!("wizard_step_title_model_fetching").to_string()
            }),
            Line::from(""),
        ];
        if !state.preset_models.is_empty() {
            for (i, m) in state.preset_models.iter().enumerate() {
                text.push(Line::from(format!("  {}. {}", i + 1, m)));
            }
            text.push(Line::from(""));
            text.push(Line::from("输入序号或手填模型 ID / Esc 取消"));
        } else {
            text.push(Line::from("(无预设模型列表)"));
            text.push(Line::from(""));
            text.push(Line::from("手填模型 ID: "));
        }
        Paragraph::new(text).render(area, buf);
    }
    fn on_key(&self, _key: KeyEvent, state: &mut WizardState) -> StepAction {
        let input = match read_line_from_stdin() {
            Some(s) if !s.is_empty() => s,
            _ => return StepAction::Stay,
        };
        if let Ok(n) = input.parse::<usize>()
            && n >= 1
            && n <= state.preset_models.len()
        {
            state.default_model = Some(state.preset_models[n - 1].clone());
            return StepAction::Next;
        }
        state.default_model = Some(input);
        StepAction::Next
    }
    fn is_complete(&self, state: &WizardState) -> bool {
        state.default_model.is_some()
    }
    fn next_step(&self) -> Option<Arc<dyn WizardStep>> {
        Some(Arc::new(super::confirm::ConfirmStep))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_complete_false_when_no_model() {
        let step = ModelStep;
        let state = WizardState::default();
        assert!(!step.is_complete(&state));
    }

    #[test]
    fn is_complete_true_when_model_set() {
        let step = ModelStep;
        let state = WizardState {
            default_model: Some("gpt-4o".into()),
            ..WizardState::default()
        };
        assert!(step.is_complete(&state));
    }
}
