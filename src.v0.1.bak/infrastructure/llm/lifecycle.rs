//! Router lifecycle — 从配置创建 LlmRouter
//!
//! V0.1.0: deepseek 单家，从配置文件或环境变量获取 API key。

use std::collections::HashMap;
use std::sync::Arc;

use crate::common::error::{EflowError, Result};
use crate::common::types::ModelTier;
use crate::infrastructure::config::EflowConfig;
use crate::infrastructure::llm::deepseek::DeepseekProvider;
use crate::infrastructure::llm::l2::L2CacheManager;

use super::router::LlmRouter;
use super::tier::TierRouter;

impl LlmRouter {
    /// 从配置创建 Router — V0.1.0 deepseek 单家
    pub fn from_config(config: &EflowConfig) -> Result<Self> {
        let ds = &config.llm.deepseek;
        let api_key = ds.api_key.clone()
            .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())
            .ok_or_else(|| EflowError::Config(
                "No DeepSeek API key configured. Set DEEPSEEK_API_KEY env var or configure in qingbird.yaml".to_string()
            ))?;

        let provider = DeepseekProvider::new(
            api_key,
            ds.base_url.clone(),
            ds.default_model
                .clone()
                .unwrap_or_else(|| "deepseek-chat".to_string()),
            ds.timeout_secs,
            ds.max_retries,
            ds.retry_backoff_ms,
        )?;

        let routing: HashMap<ModelTier, String> = [
            (ModelTier::Strong, "deepseek".into()),
            (ModelTier::Medium, "deepseek".into()),
            (ModelTier::Light, "deepseek".into()),
        ]
        .into_iter()
        .collect();

        let l2_cache = if config.llm.cache.l2_enabled {
            let path = std::path::Path::new("./data/llm_cache.db");
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match L2CacheManager::new(1000, path, config.llm.cache.l2_ttl_days) {
                Ok(m) => Some(Arc::new(m)),
                Err(e) => {
                    tracing::warn!("L2 cache init failed: {}; disabled", e);
                    None
                }
            }
        } else {
            None
        };

        let tier_router = TierRouter::new(provider, routing, l2_cache);

        Ok(Self { tier_router })
    }
}
