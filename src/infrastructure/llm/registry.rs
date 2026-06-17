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
