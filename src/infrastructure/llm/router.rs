//! LlmRouter — LLM 调用统一入口
//!
//! V0.1.0: 只有 deepseek 单 provider，router 退化为门面。

use crate::common::error::Result;
use crate::common::types::ModelTier;
use crate::infrastructure::llm::tier::TierRouter;
use crate::infrastructure::llm::types::ChatRequest;

pub struct LlmRouter {
    pub(super) tier_router: TierRouter,
}

impl LlmRouter {
    /// 非缓存聊天
    pub async fn chat(
        &self,
        tier: ModelTier,
        request: ChatRequest,
    ) -> Result<super::types::ChatResponse> {
        self.tier_router.chat(tier, request).await
    }

    /// 缓存优先聊天
    pub async fn chat_cached(
        &self,
        tier: ModelTier,
        request: ChatRequest,
        cache_key: &super::cache::CacheKey,
    ) -> Result<super::types::ChatResponse> {
        self.tier_router.chat_cached(tier, request, cache_key).await
    }

    /// 返回当前 provider 名称
    pub fn provider_name(&self) -> &str {
        "deepseek"
    }

    /// 创建空 Router（用于初始化尚未就绪的占位场景）
    ///
    /// 调用方应在有 config 后通过 `from_config` 替换。
    /// 与旧 `placeholder()` 等价的简化版本，无测试注入方法。
    ///
    /// 当前用途：concierge feedbacker 在启动时序中先占位、后替换。
    #[doc(hidden)]
    pub fn placeholder() -> Self {
        let provider = crate::infrastructure::llm::deepseek::DeepseekProvider::new(
            "placeholder".into(),
            None,
            "deepseek-chat".into(),
            30,
            3,
            1000,
        )
        .expect("placeholder provider creation should not fail");
        let routing = [
            (ModelTier::Strong, "deepseek".into()),
            (ModelTier::Medium, "deepseek".into()),
            (ModelTier::Light, "deepseek".into()),
        ]
        .into_iter()
        .collect();
        Self {
            tier_router: TierRouter::new(provider, routing, None),
        }
    }
}
