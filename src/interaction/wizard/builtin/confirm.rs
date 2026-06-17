//! step 6: 确认 + 写文件

use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::prelude::{Buffer, Widget};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::common::error::{EflowError, Result};
use crate::infrastructure::llm::types::{ProtocolKind, ProviderConfig};
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
    fn render(&self, area: Rect, buf: &mut Buffer, state: &WizardState) {
        // 临时硬编码
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

        let text = vec![
            Line::from("配置确认："),
            Line::from(""),
            Line::from(format!("  语言: {}", state.config.core.language)),
            Line::from(format!(
                "  厂商: {}",
                state.provider_display_name.as_deref().unwrap_or("(未选)")
            )),
            Line::from(format!(
                "  协议: {}",
                match state.provider_protocol {
                    Some(ProtocolKind::OpenaiCompatible) => "openai_compatible",
                    Some(ProtocolKind::AnthropicCompatible) => "anthropic_compatible",
                    None => "(preset 跳过)",
                }
            )),
            Line::from(format!("  API KEY: {}", masked_key)),
            Line::from(format!(
                "  模型: {}",
                state.default_model.as_deref().unwrap_or("(未选)")
            )),
            Line::from(""),
            Line::from("Enter 确认 / Esc 取消（不写文件）"),
        ];
        Paragraph::new(text).render(area, buf);
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
            .unwrap_or(ProtocolKind::OpenaiCompatible);
        let base_url = state
            .provider_base_url
            .clone()
            .unwrap_or_else(|| match protocol {
                ProtocolKind::OpenaiCompatible => "https://api.openai.com/v1".to_string(),
                ProtocolKind::AnthropicCompatible => "https://api.anthropic.com".to_string(),
            });
        let provider_cfg = ProviderConfig {
            id: id.clone(),
            display_name: state
                .provider_display_name
                .clone()
                .unwrap_or_else(|| id.clone()),
            protocol,
            base_url,
            api_key: state.provider_api_key.clone().unwrap_or_default(),
            default_model: state.default_model.clone().unwrap_or_default(),
            timeout_secs: 30,
            max_retries: 3,
            retry_backoff_ms: 1000,
            preset_models: state.preset_models.clone(),
            list_models_endpoint: None,
            list_models: vec![],
            extra_config: serde_json::Value::Null,
        };
        let provider_content = serde_yaml::to_string(&provider_cfg)
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
