//! step 6: 确认 + 写文件

use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};

use crate::common::error::{EflowError, Result};
use crate::interaction::render::view_model::*;
use crate::interaction::wizard::{StepAction, WizardState, WizardStep};

pub struct ConfirmStep;

#[async_trait]
impl WizardStep for ConfirmStep {
    fn id(&self) -> &'static str {
        "confirm"
    }
    fn title(&self) -> &'static str {
        "确认配置"
    }
    fn view_model(&self, state: &WizardState) -> StepViewModel {
        let masked_key = state
            .provider_api_key
            .as_ref()
            .map(|k| {
                if k.len() > 8 {
                    format!("{}***{}", &k[..4], &k[k.len() - 4..])
                } else {
                    "***".to_string()
                }
            })
            .unwrap_or_else(|| "(未填)".to_string());

        let protocol_display = match state.provider_protocol.as_deref() {
            Some("openai_compatible") => "openai_compatible",
            Some("anthropic_compatible") => "anthropic_compatible",
            _ => "(preset 跳过)",
        };

        StepViewModel {
            title: "确认配置".into(),
            lines: vec![
                LineVM {
                    text: "配置确认：".into(),
                },
                LineVM { text: "".into() },
                LineVM {
                    text: format!("  语言: {}", state.config.core.language),
                },
                LineVM {
                    text: format!(
                        "  厂商: {}",
                        state.provider_display_name.as_deref().unwrap_or("(未选)")
                    ),
                },
                LineVM {
                    text: format!("  协议: {}", protocol_display),
                },
                LineVM {
                    text: format!("  API KEY: {}", masked_key),
                },
                LineVM {
                    text: format!(
                        "  模型: {}",
                        state.default_model.as_deref().unwrap_or("(未选)")
                    ),
                },
                LineVM { text: "".into() },
                LineVM {
                    text: "Enter 确认 / Esc 取消（不写文件）".into(),
                },
            ],
            input: None,
            hints: vec![
                KeyHint {
                    key: "Enter".into(),
                    description: "确认".into(),
                },
                KeyHint {
                    key: "Esc".into(),
                    description: "取消".into(),
                },
            ],
            focused_field: 0,
        }
    }
    fn on_key(&self, key: KeyEvent, _state: &mut WizardState) -> StepAction {
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
        None
    }
    fn on_exit(&self, state: &mut WizardState) {
        // finalize：写文件
        let _ = finalize(state);
    }
}

/// 写 ~/.eflow/config.yaml + ~/.eflow/providers/{id}.yaml
///
/// 原子写：tmp 文件 + rename
fn finalize(state: &WizardState) -> Result<()> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| EflowError::Config("无法定位 home 目录".into()))?
        .join("eflow");
    std::fs::create_dir_all(&config_dir).map_err(EflowError::Io)?;
    let providers_dir = config_dir.join("providers");
    std::fs::create_dir_all(&providers_dir).map_err(EflowError::Io)?;

    let config_path = config_dir.join("config.yaml");
    let config_content = serde_yaml::to_string(&state.config)
        .map_err(|e| EflowError::Config(format!("序列化 config 失败: {e}")))?;
    atomic_write(&config_path, &config_content)?;

    if let Some(id) = &state.provider_id {
        let protocol = state
            .provider_protocol
            .clone()
            .unwrap_or_else(|| "openai_compatible".to_string());
        let base_url = state
            .provider_base_url
            .clone()
            .unwrap_or_else(|| match protocol.as_str() {
                "anthropic_compatible" => "https://api.anthropic.com".to_string(),
                _ => "https://api.openai.com/v1".to_string(),
            });
        let provider_content = serde_yaml::to_string(&serde_json::json!({
            "id": id,
            "display_name": state.provider_display_name.clone().unwrap_or_else(|| id.clone()),
            "protocol": protocol,
            "base_url": base_url,
            "api_key": state.provider_api_key.clone().unwrap_or_default(),
            "default_model": state.default_model.clone().unwrap_or_default(),
            "timeout_secs": 30,
            "max_retries": 3,
            "retry_backoff_ms": 1000,
            "preset_models": state.preset_models.clone(),
        }))
        .map_err(|e| EflowError::Config(format!("序列化 provider 失败: {e}")))?;
        atomic_write(&providers_dir.join(format!("{id}.yaml")), &provider_content)?;
    }

    Ok(())
}

/// 原子写：写 .tmp 文件 + rename
fn atomic_write(path: &std::path::Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content).map_err(EflowError::Io)?;
    std::fs::rename(&tmp, path).map_err(EflowError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn id_and_title_not_empty() {
        let step = ConfirmStep;
        assert_eq!(step.id(), "confirm");
        assert!(!step.title().is_empty());
    }

    #[test]
    fn enter_returns_next_esc_returns_cancel() {
        let step = ConfirmStep;
        let mut state = WizardState::default();
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert!(matches!(step.on_key(enter, &mut state), StepAction::Next));
        assert!(matches!(step.on_key(esc, &mut state), StepAction::Cancel));
    }

    #[test]
    fn view_model_masks_api_key() {
        let step = ConfirmStep;
        let state = WizardState {
            provider_api_key: Some("sk-1234567890abcdef".into()),
            ..WizardState::default()
        };
        let vm = step.view_model(&state);
        let text: String = vm.lines.iter().map(|l| l.text.as_str()).collect();
        // mask: k[..4]="sk-1", k[len-4..]="cdef" → "sk-1***cdef"
        assert!(text.contains("sk-1"), "should show first 4 chars");
        assert!(text.contains("***"), "should have *** in middle");
        assert!(text.contains("cdef"), "should show last 4 chars");
        assert!(!text.contains("1234567890ab"), "should not show full key");
    }

    #[test]
    fn view_model_shows_protocol_display() {
        let step = ConfirmStep;
        let state = WizardState {
            provider_protocol: Some("openai_compatible".into()),
            ..WizardState::default()
        };
        let vm = step.view_model(&state);
        let text: String = vm.lines.iter().map(|l| l.text.as_str()).collect();
        assert!(text.contains("openai_compatible"));
    }

    #[test]
    fn view_model_no_input_field() {
        let step = ConfirmStep;
        let state = WizardState::default();
        let vm = step.view_model(&state);
        assert!(vm.input.is_none(), "confirm step has no input field");
        assert!(!vm.hints.is_empty(), "should have Enter/Esc hints");
    }
}
