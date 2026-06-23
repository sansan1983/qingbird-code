//! step 5: 模型选择（自动拉取 + 手填 fallback）

use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::KeyEvent;
use rust_i18n::t;
use serde_json::Value;

use crate::interaction::render::view_model::*;
use crate::interaction::wizard::{StepAction, WizardState, WizardStep};

pub struct ModelStep;

/// 调 /models 拉取可用模型列表，合并入 preset_models
fn fetch_models(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client: {}", e))?;

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .map_err(|e| format!("request: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json: Value = resp.json().map_err(|e| format!("parse JSON: {}", e))?;

    let models = json
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "no 'data' array in response".to_string())?;

    Ok(models
        .iter()
        .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
        .collect())
}

#[async_trait]
impl WizardStep for ModelStep {
    fn id(&self) -> &'static str {
        "model"
    }
    fn title(&self) -> &'static str {
        "选择模型"
    }
    fn view_model(&self, state: &WizardState) -> StepViewModel {
        let title = if state.fetch_failed {
            t!("wizard_step_title_model_fallback").to_string()
        } else {
            t!("wizard_step_title_model_fetching").to_string()
        };

        let mut lines: Vec<LineVM> = vec![LineVM { text: "".into() }];
        if !state.preset_models.is_empty() {
            for (i, m) in state.preset_models.iter().enumerate() {
                lines.push(LineVM {
                    text: format!("  {}. {}", i + 1, m),
                });
            }
            lines.push(LineVM { text: "".into() });
            lines.push(LineVM {
                text: "输入序号或手填模型 ID / Esc 取消".into(),
            });
        } else {
            lines.push(LineVM {
                text: "(无预设模型列表)".into(),
            });
            lines.push(LineVM { text: "".into() });
            lines.push(LineVM {
                text: "手填模型 ID: ".into(),
            });
        }

        StepViewModel {
            title,
            lines,
            input: None,
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
    fn on_enter(&self, state: &mut WizardState) {
        // 拉取可用模型，合并到 preset_models
        if state.fetch_failed || state.provider_api_key.is_none() {
            return;
        }
        let base_url = match &state.provider_base_url {
            Some(u) => u.clone(),
            None => return,
        };
        let api_key = match &state.provider_api_key {
            Some(k) => k.clone(),
            None => return,
        };
        match fetch_models(&base_url, &api_key) {
            Ok(fetched) => {
                // 合并 + 去重：preset_models 在前，fetched 在后
                let mut models: Vec<String> = state.preset_models.clone();
                for m in fetched {
                    if !models.contains(&m) {
                        models.push(m);
                    }
                }
                state.preset_models = models;
            }
            Err(e) => {
                state.fetch_failed = true;
                // fetch 失败不阻塞——降级到 preset_models
                tracing::warn!("fetch /models failed: {}", e);
            }
        }
    }

    fn on_key(&self, key: KeyEvent, state: &mut WizardState) -> StepAction {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => StepAction::Cancel,
            KeyCode::Enter => {
                if state.default_model.is_some() {
                    StepAction::Next
                } else {
                    StepAction::Stay
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let n = c.to_digit(10).unwrap_or(0) as usize;
                if n >= 1 && n <= state.preset_models.len() {
                    state.default_model = Some(state.preset_models[n - 1].clone());
                    StepAction::Next
                } else {
                    StepAction::Stay
                }
            }
            _ => StepAction::Stay,
        }
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
