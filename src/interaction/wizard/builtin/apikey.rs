//! step 4: 填 API KEY

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
    fn render(&self, area: Rect, buf: &mut Buffer, _state: &WizardState) {
        // 临时硬编码
        let text = vec![
            Line::from(t!("wizard_step_title_apikey").to_string()),
            Line::from(""),
            Line::from("（v1.3.1 阶段：从 stdin 输入 KEY）"),
            Line::from(""),
            Line::from("API KEY: "),
        ];
        Paragraph::new(text).render(area, buf);
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
