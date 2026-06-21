pub mod cache;
pub mod generic_anthropic;
pub mod generic_openai;
pub mod http_client;
pub mod preset_loader;
pub mod registry;
pub mod router;
pub mod types;

pub use cache::{CacheKey, CacheValue, ContextProfile, cache_key_for_step, key_hash};
pub use router::LlmRouter;
pub use types::*;

/// 把 `request.model` 中的空字符串替换为 `default_model`（fix v1.0.3 R4 抽离）
pub(crate) fn pick_model(default_model: &str, request_model: &str) -> String {
    if request_model.is_empty() {
        default_model.to_string()
    } else {
        request_model.to_string()
    }
}
