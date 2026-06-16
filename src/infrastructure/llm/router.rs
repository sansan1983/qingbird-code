use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::common::error::{EflowError, Result};
use crate::common::types::ModelTier;
use crate::infrastructure::config::EflowConfig;
use rust_i18n::t;

use super::anthropic::AnthropicProvider;
use super::cache::{CacheKey, CacheValue, L2CacheManager};
use super::openai::OpenAiProvider;
use super::types::{ChatRequest, ChatResponse, LlmProvider, TokenUsage};

/// LLM Router — 统一入口，按 `ModelTier` 路由到具体 Provider
pub struct LlmRouter {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
    routing: HashMap<ModelTier, String>,
    rate_limit_counters: HashMap<String, u32>,
    l2_cache: Option<Arc<L2CacheManager>>,
}

impl LlmRouter {
    /// 从配置创建 Router
    pub fn from_config(config: &EflowConfig) -> Result<Self> {
        let mut providers: HashMap<String, Arc<dyn LlmProvider>> = HashMap::new();

        if let Some(ref anthro) = config.llm.providers.anthropic
            && !anthro.api_key.is_empty()
            && !anthro.api_key.starts_with("${")
        {
            // v1.1 跨阶段: 读 ANTHROPIC_BASE_URL env var（企业代理 / 第三方兼容服务）
            let base_url = std::env::var("ANTHROPIC_BASE_URL").ok();
            let provider = AnthropicProvider::with_options(
                anthro.api_key.clone(),
                anthro.default_model.clone(),
                anthro.timeout_secs,
                anthro.max_retries,
                anthro.retry_backoff_ms,
                base_url,
            );
            providers.insert("anthropic".into(), Arc::new(provider));
        }

        if let Some(ref openai) = config.llm.providers.openai
            && !openai.api_key.is_empty()
            && !openai.api_key.starts_with("${")
        {
            let base_url = std::env::var("OPENAI_BASE_URL").ok();
            let provider = OpenAiProvider::with_options(
                openai.api_key.clone(),
                openai.default_model.clone(),
                openai.timeout_secs,
                openai.max_retries,
                openai.retry_backoff_ms,
                base_url,
            );
            providers.insert("openai".into(), Arc::new(provider));
        }

        if providers.is_empty() {
            return Err(EflowError::Config(t!("err_no_llm_providers").to_string()));
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

        // v1.1 Task B5: 可选 L2 缓存（设计 §8.5）
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

        Ok(Self {
            providers,
            routing,
            rate_limit_counters: HashMap::new(),
            l2_cache,
        })
    }

    /// 按 `ModelTier` 路由调用（含指数退避重试 + 降级）
    pub async fn chat(&mut self, tier: ModelTier, request: ChatRequest) -> Result<ChatResponse> {
        let provider_name = self
            .routing
            .get(&tier)
            .ok_or_else(|| {
                EflowError::Internal(
                    t!("err_no_provider", tier = format!("{:?}", tier)).to_string(),
                )
            })?
            .clone();

        // 从 provider 实例读取重试参数（fix v1.1 Task A3 — trait default 实现）
        let (max_retries, backoff_ms) = self
            .providers
            .get(&provider_name)
            .map(|p| p.retry_params())
            .unwrap_or((3, 1000));

        // 保留一份原请求给降级路径用
        let fallback_request = request.clone();
        match self
            .chat_with_retry(tier, request, max_retries, backoff_ms)
            .await
        {
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
                    self.try_tier_degrade(tier, fallback_request, count).await
                } else {
                    Err(EflowError::RateLimited(provider_name))
                }
            }
            Err(e) => Err(e),
        }
    }

    /// 带 L2 缓存的 chat 入口（v1.1 Task B5 — 设计 §8.2 命中短路）
    pub async fn chat_cached(
        &mut self,
        tier: ModelTier,
        request: ChatRequest,
        key: &CacheKey,
    ) -> Result<ChatResponse> {
        // 1. 查 L2
        if let Some(cache) = &self.l2_cache
            && let Some(CacheValue::Execution { result_summary, .. }) = cache.lookup(key)
        {
            return Ok(ChatResponse {
                content: result_summary,
                tool_calls: None,
                usage: TokenUsage::default(),
                finish_reason: "cache_hit".into(),
            });
        }
        // 2. 调 LLM
        let resp = self.chat(tier, request).await?;
        // 3. 写 L2
        if let Some(cache) = &self.l2_cache {
            cache.store(
                key,
                CacheValue::Execution {
                    result_summary: resp.content.clone(),
                    success: true,
                    duration_ms: 0,
                },
            );
        }
        Ok(resp)
    }

    /// 带指数退避的重试封装（fix v1.1 Task A3）
    /// 对 `RateLimited` 和 `LlmProvider`（含 4xx/5xx）两类瞬时错误都重试。
    pub async fn chat_with_retry(
        &mut self,
        tier: ModelTier,
        request: ChatRequest,
        max_retries: u8,
        backoff_ms: u64,
    ) -> Result<ChatResponse> {
        let provider_name = self
            .routing
            .get(&tier)
            .ok_or_else(|| {
                EflowError::Internal(
                    t!("err_no_provider", tier = format!("{:?}", tier)).to_string(),
                )
            })?
            .clone();
        self.chat_with_retry_named(&provider_name, request, max_retries, backoff_ms)
            .await
    }

    /// 通过 provider name 直接调用重试（v1.1 Task A5：同 tier 跨 provider 降级需要
    /// 跳过 routing 映射里的失败 provider，直接命中 fallback）
    pub async fn chat_with_retry_named(
        &self,
        provider_name: &str,
        request: ChatRequest,
        max_retries: u8,
        backoff_ms: u64,
    ) -> Result<ChatResponse> {
        let provider = self
            .providers
            .get(provider_name)
            .ok_or_else(|| {
                EflowError::Internal(
                    t!("err_provider_not_found", name = provider_name.to_string()).to_string(),
                )
            })?
            .clone();

        let mut attempt = 0u8;
        loop {
            match provider.chat(request.clone()).await {
                Ok(resp) => return Ok(resp),
                Err(EflowError::RateLimited(_)) if attempt < max_retries => {
                    let delay = backoff_ms * 2u64.pow(attempt as u32);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    attempt += 1;
                }
                Err(EflowError::LlmProvider(_)) if attempt < max_retries => {
                    let delay = backoff_ms * 2u64.pow(attempt as u32);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// 模型 tier 降级路径（设计 §11.2 — v1.1 Task A5）
    /// - Strong → Medium → Light
    /// - Medium → Light
    /// - Light → 同 tier 跨 provider
    pub async fn try_tier_degrade(
        &mut self,
        original_tier: ModelTier,
        request: ChatRequest,
        rate_limit_count: u32,
    ) -> Result<ChatResponse> {
        if rate_limit_count >= 10 {
            return Err(EflowError::RateLimited(
                t!("err_all_providers_limited", count = rate_limit_count).to_string(),
            ));
        }

        // 1. 尝试降级到下一 tier
        let next_tier = match original_tier {
            ModelTier::Strong => Some(ModelTier::Medium),
            ModelTier::Medium => Some(ModelTier::Light),
            ModelTier::Light => None,
        };

        if let Some(tier) = next_tier
            && let Some(next_name) = self.routing.get(&tier).cloned()
        {
            let provider = self.providers.get(&next_name).cloned();
            if let Some(p) = provider {
                tracing::warn!(
                    "Tier degrade: {:?} → {:?} (provider={})",
                    original_tier,
                    tier,
                    next_name
                );
                let (max_retries, backoff_ms) = p.retry_params();
                return self
                    .chat_with_retry_named(&next_name, request, max_retries, backoff_ms)
                    .await;
            }
        }

        // 2. 同 tier 跨 provider 降级（plan bug fix: 必须用 named 调用，
        //    否则 chat_with_retry(tier, ...) 会再次路由回失败 provider）
        let current_name = self.routing.get(&original_tier).cloned();
        let fallback = self
            .providers
            .keys()
            .find(|name| Some(*name) != current_name.as_ref())
            .cloned();

        if let Some(fallback_name) = fallback {
            let provider = self.providers.get(&fallback_name).cloned();
            if let Some(p) = provider {
                tracing::warn!(
                    "Provider degrade (same tier): {:?} → {}",
                    original_tier,
                    fallback_name
                );
                let (max_retries, backoff_ms) = p.retry_params();
                return self
                    .chat_with_retry_named(&fallback_name, request, max_retries, backoff_ms)
                    .await;
            }
        }

        Err(EflowError::RateLimited(t!("err_no_fallback").to_string()))
    }

    /// 获取 provider 名称
    #[must_use]
    pub fn provider_for(&self, tier: ModelTier) -> Option<&str> {
        self.routing.get(&tier).map(std::string::String::as_str)
    }
}

#[cfg(test)]
impl LlmRouter {
    /// 测试用空 Router（v1.1 Task B6 — Feedbacker 单元测试用）
    #[must_use]
    pub fn placeholder() -> Self {
        Self {
            providers: HashMap::new(),
            routing: HashMap::new(),
            rate_limit_counters: HashMap::new(),
            l2_cache: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::llm::cache::L2CacheManager;
    use crate::infrastructure::llm::types::{ChatChunk, ChatResponse, Message, TokenUsage};
    use async_trait::async_trait;
    use tempfile::TempDir;

    /// 模拟前两次失败、第三次成功的 Provider
    struct FlakyProvider {
        attempts: Arc<std::sync::Mutex<u32>>,
        fail_until: u32,
    }

    #[async_trait]
    impl LlmProvider for FlakyProvider {
        async fn chat(&self, _: ChatRequest) -> Result<ChatResponse> {
            let mut a = self.attempts.lock().unwrap();
            *a += 1;
            if *a <= self.fail_until {
                Err(EflowError::LlmProvider("flaky".into()))
            } else {
                Ok(ChatResponse {
                    content: "ok".into(),
                    tool_calls: None,
                    usage: TokenUsage::default(),
                    finish_reason: "stop".into(),
                })
            }
        }
        async fn chat_stream(
            &self,
            _: ChatRequest,
        ) -> Result<tokio::sync::mpsc::Receiver<Result<ChatChunk>>> {
            Err(EflowError::Internal("n/a".into()))
        }
        fn supports_prefix_cache(&self) -> bool {
            false
        }
        fn name(&self) -> &str {
            "flaky"
        }
    }

    #[tokio::test]
    async fn router_retries_with_backoff_on_provider_error() {
        // v1.1 Task A3: 验证失败后指数退避重试并最终成功
        let attempts = Arc::new(std::sync::Mutex::new(0));
        let provider = FlakyProvider {
            attempts: attempts.clone(),
            fail_until: 2,
        };
        let mut router = LlmRouter {
            providers: HashMap::from([(
                "flaky".into(),
                Arc::new(provider) as Arc<dyn LlmProvider>,
            )]),
            routing: HashMap::from([(ModelTier::Light, "flaky".into())]),
            rate_limit_counters: HashMap::new(),
            l2_cache: None,
        };
        let req = ChatRequest::new("", vec![Message::user("hi")]);
        let resp = router
            .chat_with_retry(ModelTier::Light, req, 3, 10)
            .await
            .unwrap();
        assert_eq!(resp.content, "ok");
        // 失败 2 次 + 成功 1 次 = 3 次调用
        assert_eq!(*attempts.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn router_gives_up_after_max_retries() {
        let attempts = Arc::new(std::sync::Mutex::new(0));
        let provider = FlakyProvider {
            attempts: attempts.clone(),
            fail_until: 99,
        };
        let mut router = LlmRouter {
            providers: HashMap::from([(
                "flaky".into(),
                Arc::new(provider) as Arc<dyn LlmProvider>,
            )]),
            routing: HashMap::from([(ModelTier::Light, "flaky".into())]),
            rate_limit_counters: HashMap::new(),
            l2_cache: None,
        };
        let req = ChatRequest::new("", vec![Message::user("hi")]);
        let result = router.chat_with_retry(ModelTier::Light, req, 2, 1).await;
        assert!(result.is_err());
        // 初次 + 2 次重试 = 3 次调用
        assert_eq!(*attempts.lock().unwrap(), 3);
    }

    // ========== v1.1 Task A5: tier 降级路径 ==========

    struct AlwaysLimited;
    #[async_trait]
    impl LlmProvider for AlwaysLimited {
        async fn chat(&self, _: ChatRequest) -> Result<ChatResponse> {
            Err(EflowError::RateLimited("limited".into()))
        }
        async fn chat_stream(
            &self,
            _: ChatRequest,
        ) -> Result<tokio::sync::mpsc::Receiver<Result<ChatChunk>>> {
            Err(EflowError::Internal("n/a".into()))
        }
        fn supports_prefix_cache(&self) -> bool {
            false
        }
        fn name(&self) -> &str {
            "limited"
        }
    }

    struct OkProvider;
    #[async_trait]
    impl LlmProvider for OkProvider {
        async fn chat(&self, _: ChatRequest) -> Result<ChatResponse> {
            Ok(ChatResponse {
                content: "fallback-ok".into(),
                tool_calls: None,
                usage: TokenUsage::default(),
                finish_reason: "stop".into(),
            })
        }
        async fn chat_stream(
            &self,
            _: ChatRequest,
        ) -> Result<tokio::sync::mpsc::Receiver<Result<ChatChunk>>> {
            Err(EflowError::Internal("n/a".into()))
        }
        fn supports_prefix_cache(&self) -> bool {
            false
        }
        fn name(&self) -> &str {
            "ok"
        }
    }

    #[tokio::test]
    async fn tier_degrade_falls_back_to_lower_tier() {
        // v1.1 Task A5: Strong 限流 → 降级到 Medium
        let mut router = LlmRouter {
            providers: HashMap::from([
                (
                    "limited".into(),
                    Arc::new(AlwaysLimited) as Arc<dyn LlmProvider>,
                ),
                ("ok".into(), Arc::new(OkProvider) as Arc<dyn LlmProvider>),
            ]),
            routing: HashMap::from([
                (ModelTier::Strong, "limited".into()),
                (ModelTier::Medium, "ok".into()),
                (ModelTier::Light, "ok".into()),
            ]),
            rate_limit_counters: HashMap::from([("limited".into(), 5)]),
            l2_cache: None,
        };
        let req = ChatRequest::new("", vec![Message::user("hi")]);
        let resp = router
            .try_tier_degrade(ModelTier::Strong, req, 5)
            .await
            .unwrap();
        assert_eq!(resp.content, "fallback-ok");
    }

    #[tokio::test]
    async fn tier_degrade_same_tier_falls_back_to_other_provider() {
        // v1.1 Task A5: Light 限流（同 tier 已无更低端）→ 跨 provider 降级
        // 验证 bug fix：不能再次路由回 limited provider
        let mut router = LlmRouter {
            providers: HashMap::from([
                (
                    "limited".into(),
                    Arc::new(AlwaysLimited) as Arc<dyn LlmProvider>,
                ),
                ("ok".into(), Arc::new(OkProvider) as Arc<dyn LlmProvider>),
            ]),
            routing: HashMap::from([
                (ModelTier::Strong, "limited".into()),
                (ModelTier::Medium, "limited".into()),
                (ModelTier::Light, "limited".into()),
            ]),
            rate_limit_counters: HashMap::from([("limited".into(), 5)]),
            l2_cache: None,
        };
        let req = ChatRequest::new("", vec![Message::user("hi")]);
        let resp = router
            .try_tier_degrade(ModelTier::Light, req, 5)
            .await
            .unwrap();
        assert_eq!(resp.content, "fallback-ok");
    }

    // ========== v1.1 Task B5: L2 缓存接入 ==========

    #[tokio::test]
    async fn router_serves_from_l2_cache_on_second_call() {
        use crate::common::types::{IntentType, RiskLevel};
        use crate::infrastructure::llm::cache::{CacheKey, CacheValue, ContextProfile};

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("cache.db");
        let cache = Arc::new(L2CacheManager::new(100, &path, 7).unwrap());

        let key = CacheKey {
            intent_type: IntentType::CodeReview,
            task_signature: "sig".into(),
            context_profile: ContextProfile {
                conversation_depth_bucket: 0,
                file_count_bucket: 0,
                risk_level: RiskLevel::L0,
                profile_name: "dev".into(),
            },
            model: "m".into(),
        };
        // 预填充 cache
        cache.store(
            &key,
            CacheValue::Execution {
                result_summary: "cached".into(),
                success: true,
                duration_ms: 1,
            },
        );

        let mut router = LlmRouter {
            providers: HashMap::from([("ok".into(), Arc::new(OkProvider) as Arc<dyn LlmProvider>)]),
            routing: HashMap::from([(ModelTier::Light, "ok".into())]),
            rate_limit_counters: HashMap::new(),
            l2_cache: Some(cache.clone()),
        };
        let req = ChatRequest::new("", vec![Message::user("hi")]);
        let resp = router
            .chat_cached(ModelTier::Light, req, &key)
            .await
            .unwrap();
        assert_eq!(resp.content, "cached");
    }
}
