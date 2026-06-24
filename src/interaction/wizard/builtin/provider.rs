//! step 2: 厂商选择（4 个 preset + 自定义）
//!
//! v1.4+ 改进：选预设后自动填充 base_url / protocol / preset_models，
//! 不再让后续步骤从 YAML 文件读取。

use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use rust_i18n::t;

use crate::interaction::render::view_model::*;
use crate::interaction::wizard::{StepAction, WizardState, WizardStep};

pub struct ProviderStep;

pub struct PresetProvider {
    pub id: &'static str,
    pub display_name: &'static str,
    pub protocol: &'static str,
    pub base_url: &'static str,
    pub default_model: &'static str,
    pub preset_models: &'static [&'static str],
}

pub const PRESETS: &[PresetProvider] = &[
    PresetProvider {
        id: "deepseek",
        display_name: "DeepSeek",
        protocol: "openai_compatible",
        base_url: "https://api.deepseek.com",
        default_model: "deepseek-v4-pro",
        preset_models: &["deepseek-v4-pro", "deepseek-v4-flash"],
    },
    PresetProvider {
        id: "minimax",
        display_name: "MiniMax",
        protocol: "openai_compatible",
        base_url: "https://api.minimaxi.com/v1",
        default_model: "MiniMax-M3",
        preset_models: &[
            "MiniMax-M3",
            "MiniMax-M2.7",
            "MiniMax-M2.5",
            "MiniMax-M2.1",
            "MiniMax-M2",
        ],
    },
    PresetProvider {
        id: "agnes-ai",
        display_name: "Agnes AI",
        protocol: "openai_compatible",
        base_url: "https://apihub.agnes-ai.com/v1",
        default_model: "agnes-2.0-flash",
        preset_models: &["agnes-2.0-flash"],
    },
    PresetProvider {
        id: "opencode-go",
        display_name: "OpenCode Go",
        protocol: "openai_compatible",
        base_url: "https://opencode.ai/zen/go/v1",
        default_model: "glm-5.1",
        preset_models: &[
            "glm-5.1",
            "glm-5",
            "kimi-k2.7",
            "kimi-k2.6",
            "deepseek-v4-pro",
            "deepseek-v4-flash",
            "mimo-v2.5",
            "mimo-v2.5-pro",
            "minimax-m3",
            "minimax-m2.7",
            "minimax-m2.5",
            "qwen3.7-max",
            "qwen3.7-plus",
            "qwen3.6-plus",
        ],
    },
    PresetProvider {
        id: "anthropic",
        display_name: "Anthropic",
        protocol: "anthropic_compatible",
        base_url: "https://api.anthropic.com",
        default_model: "claude-sonnet-4-6",
        preset_models: &["claude-sonnet-4-6", "claude-opus-4-8", "claude-haiku-4-5"],
    },
    PresetProvider {
        id: "openai",
        display_name: "OpenAI",
        protocol: "openai_compatible",
        base_url: "https://api.openai.com/v1",
        default_model: "gpt-4o",
        preset_models: &["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo"],
    },
];

/// preset YAML 文件来源（4 个 + 自定义 = 5 选项）
fn list_providers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("deepseek", "DeepSeek"),
        ("minimax", "MiniMax"),
        ("agnes-ai", "Agnes AI"),
        ("opencode-go", "OpenCode Go"),
        ("anthropic", "Anthropic"),
        ("openai", "OpenAI"),
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
    fn view_model(&self, _state: &WizardState) -> StepViewModel {
        let mut lines: Vec<String> = vec!["".into()];
        for (i, (id, name)) in list_providers().iter().enumerate() {
            let hint = if *id == "custom" {
                t!("wizard_provider_hint_custom").to_string()
            } else {
                t!("wizard_provider_hint_preset").to_string()
            };
            lines.push(format!("  {}. {} {}", i + 1, name, hint));
        }
        lines.push("".into());
        lines.push("输入序号选择 / Esc 取消".into());

        StepViewModel {
            title: t!("wizard_step_title_provider").to_string(),
            lines: lines.into_iter().map(|s| LineVM { text: s }).collect(),
            input: None,
            hints: vec![
                KeyHint {
                    key: "1-7".into(),
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
        let n = match key.code {
            KeyCode::Char('1') => 1,
            KeyCode::Char('2') => 2,
            KeyCode::Char('3') => 3,
            KeyCode::Char('4') => 4,
            KeyCode::Char('5') => 5,
            KeyCode::Char('6') => 6,
            KeyCode::Char('7') => 7,
            KeyCode::Esc => return StepAction::Cancel,
            _ => return StepAction::Stay,
        };
        let (id, _name) = list_providers()[n - 1];
        if id == "custom" {
            // 自定义路径：标记需要 protocol 步
            state.skip_protocol_step = false;
            state.provider_id = Some("custom".into());
            state.provider_display_name = Some("Custom".into());
        } else if let Some(preset) = PRESETS.iter().find(|p| p.id == id) {
            // preset 路径：从内嵌数据填充 base_url / protocol / preset_models
            state.skip_protocol_step = true;
            state.provider_id = Some(preset.id.into());
            state.provider_display_name = Some(preset.display_name.into());
            state.provider_protocol = Some(preset.protocol.to_string());
            state.provider_base_url = Some(preset.base_url.into());
            state.default_model = Some(preset.default_model.into());
            state.preset_models = preset.preset_models.iter().map(|m| m.to_string()).collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn char_1_selects_deepseek_and_skips_protocol() {
        let step = ProviderStep;
        let mut state = WizardState::default();
        let key = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
        let action = step.on_key(key, &mut state);
        assert!(matches!(action, StepAction::Next));
        assert_eq!(state.provider_id.as_deref(), Some("deepseek"));
        assert!(state.skip_protocol_step);
        // v1.4+：预设数据应被填充
        assert_eq!(
            state.provider_base_url.as_deref(),
            Some("https://api.deepseek.com")
        );
        assert_eq!(state.default_model.as_deref(), Some("deepseek-v4-pro"));
        assert!(state.preset_models.contains(&"deepseek-v4-pro".to_string()));
        assert!(
            state
                .preset_models
                .contains(&"deepseek-v4-flash".to_string())
        );
    }

    #[test]
    fn char_7_selects_custom_and_does_not_skip_protocol() {
        let step = ProviderStep;
        let mut state = WizardState::default();
        let key = KeyEvent::new(KeyCode::Char('7'), KeyModifiers::NONE);
        let action = step.on_key(key, &mut state);
        assert!(matches!(action, StepAction::Next));
        assert_eq!(state.provider_id.as_deref(), Some("custom"));
        assert!(!state.skip_protocol_step);
    }
}
