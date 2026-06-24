//! Tier routing — 按 ModelTier 路由到具体 provider
//!
//! V0.1.0: 只有 deepseek 单 provider，但保留 tier 语义（未来加新 provider 用）

use std::collections::HashMap;
use std::sync::Arc;

use crate::common::error::Result;
use crate::common::types::ModelTier;
use crate::infrastructure::llm::cache::CacheKey;
use crate::infrastructure::llm::deepseek::DeepseekProvider;
use crate::infrastructure::llm::l2::L2CacheManager;
use crate::infrastructure::llm::types::{ChatRequest, LlmProvider};

/// Tier router — V0.1.0 只有 deepseek，但保留 tier 语义（未来加新 provider 用）
pub struct TierRouter {
    /// 唯一 provider（V0.1.0 deepseek）
    pub(super) provider: DeepseekProvider,
    /// ModelTier → provider_name 映射（V0.1.0 全指向 "deepseek"）
    routing: HashMap<ModelTier, String>,
    l2_cache: Option<Arc<L2CacheManager>>,
}

impl TierRouter {
    pub fn new(
        provider: DeepseekProvider,
        routing: HashMap<ModelTier, String>,
        l2_cache: Option<Arc<L2CacheManager>>,
    ) -> Self {
        Self {
            provider,
            routing,
            l2_cache,
        }
    }

    pub async fn chat(
        &self,
        tier: ModelTier,
        request: ChatRequest,
    ) -> Result<super::types::ChatResponse> {
        let _ = self
            .routing
            .get(&tier)
            .cloned()
            .unwrap_or_else(|| "deepseek".to_string());
        self.provider.chat(request).await
    }

    pub async fn chat_cached(
        &self,
        tier: ModelTier,
        request: ChatRequest,
        cache_key: &CacheKey,
    ) -> Result<super::types::ChatResponse> {
        let _ = self
            .routing
            .get(&tier)
            .cloned()
            .unwrap_or_else(|| "deepseek".to_string());

        // V0.1.0: 简化实现，跳过 L1/L2 cache 查询。
        // Task 6 恢复 cache 集成（L1→L2→provider.chat()→write back）
        let _ = cache_key;
        let _ = &self.l2_cache;
        self.provider.chat(request).await
    }

    /// 返回当前 provider 名称
    pub fn provider_name(&self) -> &str {
        "deepseek"
    }
}
