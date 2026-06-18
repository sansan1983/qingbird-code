//! step 0: 欢迎 + 提示

use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::prelude::{Buffer, Widget};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

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
    fn render(&self, area: Rect, buf: &mut Buffer, _state: &WizardState) {
        // 临时硬编码——v1.4 spec D 重构
        let text = vec![
            Line::from(""),
            Line::from("欢迎使用 eflow — 首次配置向导"),
            Line::from(""),
            Line::from("本向导将引导你完成："),
            Line::from("  1. 选择语言（中/英）"),
            Line::from("  2. 选择 LLM 厂商（4 家预置 + 自定义）"),
            Line::from("  3. 选择兼容接口（仅自定义时显示）"),
            Line::from("  4. 输入 API KEY"),
            Line::from("  5. 选择模型（自动拉取 + 手填 fallback）"),
            Line::from("  6. 确认配置"),
            Line::from(""),
            Line::from("按 Enter 开始 / Esc 取消"),
        ];
        Paragraph::new(text).render(area, buf);
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
}
