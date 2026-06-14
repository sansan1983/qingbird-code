use std::collections::HashMap;
use std::sync::Arc;

use crate::common::error::{EflowError, Result};
use crate::common::types::ModelTier;
use crate::infrastructure::config::EflowConfig;

use super::anthropic::AnthropicProvider;
use super::openai::OpenAiProvider;
use super::types::{ChatRequest, ChatResponse, LlmProvider, Message};

/// LLM Router — 统一入口，按 ModelTier 路由到具体 Provider
pub struct LlmRouter {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
    routing: HashMap<ModelTier, String>,
    rate_limit_counters: HashMap<String, u32>,
}

impl LlmRouter {
    /// 从配置创建 Router
    pub fn from_config(config: &EflowConfig) -> Result<Self> {
        let mut providers: HashMap<String, Arc<dyn LlmProvider>> = HashMap::new();

        if let Some(ref anthro) = config.llm.providers.anthropic
            && !anthro.api_key.is_empty()
            && !anthro.api_key.starts_with("${")
        {
            let provider =
                AnthropicProvider::new(anthro.api_key.clone(), anthro.default_model.clone());
            providers.insert("anthropic".into(), Arc::new(provider));
        }

        if let Some(ref openai) = config.llm.providers.openai
            && !openai.api_key.is_empty()
            && !openai.api_key.starts_with("${")
        {
            let provider =
                OpenAiProvider::new(openai.api_key.clone(), openai.default_model.clone());
            providers.insert("openai".into(), Arc::new(provider));
        }

        if providers.is_empty() {
            return Err(EflowError::Config(
                "No LLM providers configured. Set ANTHROPIC_API_KEY or OPENAI_API_KEY.".into(),
            ));
        }

        let mut routing = HashMap::new();
        let strong = config.llm.routing.strong.clone();
        let medium = config.llm.routing.medium.clone();
        let light = config.llm.routing.light.clone();

        if providers.contains_key(&strong) {
            routing.insert(ModelTier::Strong, strong);
        }
        if providers.contains_key(&medium) {
            routing.insert(ModelTier::Medium, medium);
        }
        if providers.contains_key(&light) {
            routing.insert(ModelTier::Light, light);
        }

        for tier in [ModelTier::Strong, ModelTier::Medium, ModelTier::Light] {
            if !routing.contains_key(&tier)
                && let Some(name) = providers.keys().next()
            {
                routing.insert(tier, name.clone());
            }
        }

        Ok(Self {
            providers,
            routing,
            rate_limit_counters: HashMap::new(),
        })
    }

    /// 按 ModelTier 路由调用
    pub async fn chat(&mut self, tier: ModelTier, request: ChatRequest) -> Result<ChatResponse> {
        let provider_name = self
            .routing
            .get(&tier)
            .ok_or_else(|| EflowError::Internal(format!("No provider for tier {:?}", tier)))?
            .clone();

        let provider = self
            .providers
            .get(&provider_name)
            .ok_or_else(|| EflowError::Internal(format!("Provider '{}' not found", provider_name)))?
            .clone();

        match provider.chat(request).await {
            Ok(response) => {
                self.rate_limit_counters.remove(&provider_name);
                Ok(response)
            }
            Err(EflowError::RateLimited(_)) => {
                let count = {
                    let entry = self
                        .rate_limit_counters
                        .entry(provider_name.clone())
                        .or_insert(0);
                    *entry += 1;
                    *entry
                };

                if count >= 5 {
                    self.try_degraded_call(tier, provider_name, count).await
                } else {
                    Err(EflowError::RateLimited(provider_name))
                }
            }
            Err(e) => Err(e),
        }
    }

    /// 当主 provider 被限流时尝试降级
    async fn try_degraded_call(
        &mut self,
        _original_tier: ModelTier,
        failed_provider: String,
        rate_limit_count: u32,
    ) -> Result<ChatResponse> {
        if rate_limit_count >= 10 {
            return Err(EflowError::RateLimited(format!(
                "All providers rate limited (attempts: {})",
                rate_limit_count
            )));
        }

        let fallback = self
            .providers
            .keys()
            .find(|name| **name != failed_provider)
            .cloned();

        if let Some(fallback_name) = fallback {
            let provider = self.providers.get(&fallback_name).unwrap().clone();
            let request = ChatRequest::new(provider.name(), vec![Message::user("retry")]);
            provider.chat(request).await
        } else {
            Err(EflowError::RateLimited(
                "No fallback provider available".into(),
            ))
        }
    }

    /// 获取 provider 名称
    pub fn provider_for(&self, tier: ModelTier) -> Option<&str> {
        self.routing.get(&tier).map(|s| s.as_str())
    }
}
