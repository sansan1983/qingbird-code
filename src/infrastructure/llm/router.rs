use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::common::error::{EflowError, Result};
use crate::common::types::ModelTier;
use crate::infrastructure::config::EflowConfig;
use rust_i18n::t;

use super::cache::{CacheKey, CacheValue, L2CacheManager};
use super::generic_anthropic::GenericAnthropicProvider;
use super::generic_openai::GenericOpenAiProvider;
use super::preset_loader::PresetLoader;
use super::registry::LlmProviderRegistry;
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
    ///
    /// v1.3 改造：扫 `~/.eflow/providers/*.yaml` 加载 N 个 provider，
    /// `providers` 配置块已弃用（旧 `AnthropicProvider`/`OpenAiProvider` struct 也不再使用）。
    /// 退化路径：`providers/` 为空时从 `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` env var 构造。
    pub fn from_config(config: &EflowConfig, provider_dir: &Path) -> Result<Self> {
        // 1. 扫目录加载 presets
        let presets = PresetLoader::load_all(provider_dir)?;
        let mut providers = LlmProviderRegistry::build(presets)?;

        // 2. 退化路径：providers 为空时用 v1.2 的 env var 行为
        if providers.is_empty() {
            providers = Self::fallback_from_env_vars()?;
        }

        // 3. 全部为空 → 报错
        if providers.is_empty() {
            return Err(EflowError::Config(t!("err_no_llm_providers").to_string()));
        }

        // 4. 构造 routing（routing 引用校验 + 降级）
        let mut routing = HashMap::new();
        let strong = config.llm.routing.strong.clone();
        let medium = config.llm.routing.medium.clone();
        let light = config.llm.routing.light.clone();

        let try_route = |tier_name: &str, id: String| {
            if providers.contains_key(&id) {
                Some(id)
            } else {
                let fallback = providers.keys().next().cloned();
                if let Some(ref fb) = fallback {
                    tracing::warn!(
                        "{}",
                        t!(
                            "warn_routing_unknown_tier",
                            tier = tier_name,
                            id = id.clone(),
                            fallback = fb.clone()
                        )
                    );
                }
                fallback
            }
        };

        if let Some(id) = try_route("strong", strong) {
            routing.insert(ModelTier::Strong, id);
        }
        if let Some(id) = try_route("medium", medium) {
            routing.insert(ModelTier::Medium, id);
        }
        if let Some(id) = try_route("light", light) {
            routing.insert(ModelTier::Light, id);
        }

        // 5. 如果某 tier 还没填，用第一个可用
        for tier in [ModelTier::Strong, ModelTier::Medium, ModelTier::Light] {
            if !routing.contains_key(&tier)
                && let Some(name) = providers.keys().next()
            {
                routing.insert(tier, name.clone());
            }
        }

        // 6. L2 cache（不变）
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

    /// 退化路径：当 `~/.eflow/providers/` 为空时，从 env var 构造 anthropic + openai
    ///
    /// v1.2 行为：读 `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` + `ANTHROPIC_BASE_URL` / `OPENAI_BASE_URL` 构造 provider。
    fn fallback_from_env_vars() -> Result<HashMap<String, Arc<dyn LlmProvider>>> {
        let mut providers = HashMap::new();

        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY")
            && !api_key.is_empty()
            && !api_key.starts_with("${")
        {
            let base_url = std::env::var("ANTHROPIC_BASE_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com".to_string());
            let provider = GenericAnthropicProvider::new(
                "anthropic".into(),
                api_key,
                base_url,
                "claude-sonnet-4-6".into(),
                30,
                3,
                1000,
                HashMap::new(),
            )
            .map(Arc::new)
            .unwrap();
            providers.insert("anthropic".into(), provider as Arc<dyn LlmProvider>);
        }

        if let Ok(api_key) = std::env::var("OPENAI_API_KEY")
            && !api_key.is_empty()
            && !api_key.starts_with("${")
        {
            let base_url = std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
            let provider = GenericOpenAiProvider::new(
                "openai".into(),
                api_key,
                base_url,
                "gpt-4o".into(),
                30,
                3,
                1000,
                HashMap::new(),
            )
            .map(Arc::new)
            .unwrap();
            providers.insert("openai".into(), provider as Arc<dyn LlmProvider>);
        }

        Ok(providers)
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

    /// v1.3.1 增量：取指定 provider 的 preset_models
    ///
    /// v1.3.0 router 不持有 preset_models map，本方法 v1.3.1 阶段先返回 None。
    /// `/model` 命令会显示空列表，等 spec B 后续实施时再补完整 model cache。
    #[must_use]
    pub fn preset_models_for(&self, _provider_id: &str) -> Option<Vec<String>> {
        None
    }
}

/// v1.2 E6: 测试用——构造空 Router（unit + integration test 都可见，
/// 集成测试在独立 crate 看不到 #[cfg(test)] impl，所以不 cfg）。
/// **非测试代码不应调用**——用 `LlmRouter::from_config`。
#[doc(hidden)]
impl LlmRouter {
    #[must_use]
    pub fn placeholder() -> Self {
        Self {
            providers: HashMap::new(),
            routing: HashMap::new(),
            rate_limit_counters: HashMap::new(),
            l2_cache: None,
        }
    }

    /// v1.2 E6: 测试用——往 router 注入一个 provider。
    /// **非测试代码不应调用**。
    #[doc(hidden)]
    pub fn inject_test_provider(&mut self, name: String, provider: Arc<dyn LlmProvider>) {
        self.providers.insert(name, provider);
    }

    /// v1.2 E6: 测试用——设置 tier→provider 路由。
    /// **非测试代码不应调用**。
    #[doc(hidden)]
    pub fn inject_test_routing(&mut self, tier: ModelTier, name: String) {
        self.routing.insert(tier, name);
    }

    /// v1.3 T14/T15: 测试用——查询 provider 是否存在。
    /// **非测试代码不应调用**。
    #[doc(hidden)]
    pub fn has_test_provider(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }

    /// v1.3 T15: 测试用——查询某 tier 的 routing。
    /// **非测试代码不应调用**。
    #[doc(hidden)]
    pub fn routing_for_test(&self, tier: ModelTier) -> Option<String> {
        self.routing.get(&tier).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::types::RiskLevel;
    use crate::infrastructure::config::{
        CacheConfig, CoreConfig, LlmConfig, MemoryConfig, ProfileListConfig, RoutingConfig,
        SecurityConfig,
    };
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

    // ========== v1.3 Task T14: 退化路径 ==========

    #[tokio::test]
    async fn router_empty_provider_dir_falls_back_to_env_vars() {
        // v1.3 Task T14: provider dir 为空时，从 env var 读 anthropic + openai
        unsafe {
            std::env::set_var("ANTHROPIC_API_KEY", "sk-test");
            std::env::set_var("OPENAI_API_KEY", "sk-test-openai");
        }
        let dir = tempfile::TempDir::new().unwrap();
        let config = EflowConfig {
            core: CoreConfig {
                language: "zh-CN".into(),
                timezone: "Asia/Shanghai".into(),
            },
            llm: LlmConfig {
                routing: RoutingConfig {
                    strong: "anthropic".into(),
                    medium: "anthropic".into(),
                    light: "openai".into(),
                },
                cache: CacheConfig {
                    l1_enabled: true,
                    l2_enabled: false,
                    l2_ttl_days: 7,
                },
            },
            memory: MemoryConfig {
                working_memory_limit: 1000,
                project_db_path: "./data/project.db".into(),
                user_db_path: "./data/user.db".into(),
                cleanup_interval_hours: 24,
            },
            security: SecurityConfig {
                risk_threshold: RiskLevel::L2,
                allowed_paths: vec![],
            },
            profiles: ProfileListConfig {
                default: "developer".into(),
                available: vec![],
            },
        };
        let router = LlmRouter::from_config(&config, dir.path()).unwrap();
        assert!(router.has_test_provider("anthropic"));
        assert!(router.has_test_provider("openai"));
    }

    #[tokio::test]
    async fn router_provider_dir_with_files_skips_env_vars() {
        // v1.3 Task T14: dir 有 provider 时不读 env var
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("custom.yaml"),
            r#"
id: custom
display_name: Custom
protocol: openai_compatible
base_url: https://custom.example.com
api_key: "k"
default_model: m
"#,
        )
        .unwrap();

        let config = EflowConfig {
            core: CoreConfig {
                language: "zh-CN".into(),
                timezone: "Asia/Shanghai".into(),
            },
            llm: LlmConfig {
                routing: RoutingConfig {
                    strong: "custom".into(),
                    medium: "custom".into(),
                    light: "custom".into(),
                },
                cache: CacheConfig {
                    l1_enabled: true,
                    l2_enabled: false,
                    l2_ttl_days: 7,
                },
            },
            memory: MemoryConfig {
                working_memory_limit: 1000,
                project_db_path: "./data/project.db".into(),
                user_db_path: "./data/user.db".into(),
                cleanup_interval_hours: 24,
            },
            security: SecurityConfig {
                risk_threshold: RiskLevel::L2,
                allowed_paths: vec![],
            },
            profiles: ProfileListConfig {
                default: "developer".into(),
                available: vec![],
            },
        };
        let router = LlmRouter::from_config(&config, dir.path()).unwrap();
        assert!(router.has_test_provider("custom"));
        // 不应该读 env var，所以 anthropic / openai 都不在
        assert!(!router.has_test_provider("anthropic"));
        assert!(!router.has_test_provider("openai"));
    }

    // ========== v1.3 Task T15: routing 引用校验 ==========

    #[tokio::test]
    async fn router_unknown_routing_id_degrades_to_first_provider() {
        // v1.3 Task T15: routing 引用不存在的 id → 降级到第一个可用 + warn
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("first.yaml"),
            r#"
id: first
display_name: First
protocol: openai_compatible
base_url: https://first.example.com
api_key: "k"
default_model: m
"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("second.yaml"),
            r#"
id: second
display_name: Second
protocol: openai_compatible
base_url: https://second.example.com
api_key: "k"
default_model: m
"#,
        )
        .unwrap();

        let config = EflowConfig {
            core: CoreConfig {
                language: "zh-CN".into(),
                timezone: "Asia/Shanghai".into(),
            },
            llm: LlmConfig {
                routing: RoutingConfig {
                    strong: "nonexistent".into(), // 引用不存在的
                    medium: "second".into(),
                    light: "nonexistent".into(),
                },
                cache: CacheConfig {
                    l1_enabled: true,
                    l2_enabled: false,
                    l2_ttl_days: 7,
                },
            },
            memory: MemoryConfig {
                working_memory_limit: 1000,
                project_db_path: "./data/project.db".into(),
                user_db_path: "./data/user.db".into(),
                cleanup_interval_hours: 24,
            },
            security: SecurityConfig {
                risk_threshold: RiskLevel::L2,
                allowed_paths: vec![],
            },
            profiles: ProfileListConfig {
                default: "developer".into(),
                available: vec![],
            },
        };
        let router = LlmRouter::from_config(&config, dir.path()).unwrap();
        // strong 和 light 降级到某个可用 provider（HashMap 迭代顺序非确定）
        let strong = router.routing_for_test(ModelTier::Strong);
        assert!(
            strong == Some("first".to_string()) || strong == Some("second".to_string()),
            "Strong should degrade to an available provider, got {strong:?}"
        );
        let light = router.routing_for_test(ModelTier::Light);
        assert!(
            light == Some("first".to_string()) || light == Some("second".to_string()),
            "Light should degrade to an available provider, got {light:?}"
        );
        // medium 是合法的 second
        assert_eq!(
            router.routing_for_test(ModelTier::Medium),
            Some("second".to_string())
        );
    }
}
