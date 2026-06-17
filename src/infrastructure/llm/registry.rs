//! v1.3 LLM 抽象扩展：把 ProviderConfig 列表转成 provider 实例
//!
//! 按 protocol 字段构造对应的 Generic adapter。
//! **不**在这里查 HTTP——list_models 拉取在 Generic provider 内部做。

use std::collections::HashMap;
use std::sync::Arc;

use crate::common::error::Result;
use crate::infrastructure::llm::LlmProvider;
use crate::infrastructure::llm::generic_anthropic::GenericAnthropicProvider;
use crate::infrastructure::llm::generic_openai::GenericOpenAiProvider;
use crate::infrastructure::llm::types::{ModelEntry, ProtocolKind, ProviderConfig};

pub struct LlmProviderRegistry;

impl LlmProviderRegistry {
    /// 把 `Vec<ProviderConfig>` 转成 `HashMap<id, Arc<dyn LlmProvider>>`
    pub fn build(presets: Vec<ProviderConfig>) -> Result<HashMap<String, Arc<dyn LlmProvider>>> {
        let mut providers = HashMap::new();

        for cfg in presets {
            // 把 list_models 数组转成 HashMap<id, endpoint>
            let model_endpoints: HashMap<String, String> = cfg
                .list_models
                .iter()
                .filter_map(|m: &ModelEntry| {
                    m.endpoint.as_ref().map(|ep| (m.id.clone(), ep.clone()))
                })
                .collect();

            let provider: Arc<dyn LlmProvider> = match cfg.protocol {
                ProtocolKind::OpenaiCompatible => Arc::new(GenericOpenAiProvider::new(
                    cfg.id.clone(),
                    cfg.api_key,
                    cfg.base_url,
                    cfg.default_model,
                    cfg.timeout_secs,
                    cfg.max_retries,
                    cfg.retry_backoff_ms,
                    model_endpoints,
                )),
                ProtocolKind::AnthropicCompatible => Arc::new(GenericAnthropicProvider::new(
                    cfg.id.clone(),
                    cfg.api_key,
                    cfg.base_url,
                    cfg.default_model,
                    cfg.timeout_secs,
                    cfg.max_retries,
                    cfg.retry_backoff_ms,
                    model_endpoints,
                )),
            };

            providers.insert(cfg.id, provider);
        }

        Ok(providers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::llm::types::ProtocolKind;

    fn make_config(id: &str, protocol: ProtocolKind) -> ProviderConfig {
        ProviderConfig {
            id: id.to_string(),
            display_name: id.to_string(),
            protocol,
            base_url: "https://api.test.com".to_string(),
            api_key: "k".to_string(),
            default_model: "m".to_string(),
            timeout_secs: 30,
            max_retries: 3,
            retry_backoff_ms: 1000,
            preset_models: vec!["m".to_string()],
            list_models_endpoint: None,
            list_models: vec![],
            extra_config: serde_json::Value::Null,
        }
    }

    #[test]
    fn build_empty_list() {
        let providers = LlmProviderRegistry::build(vec![]).unwrap();
        assert!(providers.is_empty());
    }

    #[test]
    fn build_openai_compatible_provider() {
        let cfg = make_config("deepseek", ProtocolKind::OpenaiCompatible);
        let providers = LlmProviderRegistry::build(vec![cfg]).unwrap();
        assert_eq!(providers.len(), 1);
        assert!(providers.contains_key("deepseek"));
        assert_eq!(providers["deepseek"].name(), "deepseek");
    }

    #[test]
    fn build_anthropic_compatible_provider() {
        let cfg = make_config("minimax", ProtocolKind::AnthropicCompatible);
        let providers = LlmProviderRegistry::build(vec![cfg]).unwrap();
        assert!(providers.contains_key("minimax"));
        assert_eq!(providers["minimax"].name(), "minimax");
    }

    #[test]
    fn build_with_list_models_endpoints() {
        let mut cfg = make_config("opencode-go", ProtocolKind::OpenaiCompatible);
        cfg.list_models = vec![
            ModelEntry {
                id: "glm-5.1".to_string(),
                endpoint: Some("/v1/chat/completions".to_string()),
            },
            ModelEntry {
                id: "kimi-k2.7".to_string(),
                endpoint: Some("/v1/messages".to_string()),
            },
        ];
        let providers = LlmProviderRegistry::build(vec![cfg]).unwrap();
        assert!(providers.contains_key("opencode-go"));
    }
}
