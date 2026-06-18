pub mod cache;
pub mod generic_anthropic;
pub mod generic_openai;
pub mod preset_loader;
pub mod registry;
pub mod router;
pub mod types;

pub use cache::{CacheKey, CacheValue, ContextProfile, cache_key_for_step, key_hash};
pub use router::LlmRouter;
pub use types::*;

use crate::common::error::{EflowError, Result};
use rust_i18n::t;

/// 把 `request.model` 中的空字符串替换为 `default_model`（fix v1.0.3 R4 抽离）
pub(crate) fn pick_model(default_model: &str, request_model: &str) -> String {
    if request_model.is_empty() {
        default_model.to_string()
    } else {
        request_model.to_string()
    }
}

/// 检查 HTTP 响应状态码：401→AuthFailed, 429→RateLimited, 其它 4xx/5xx→LlmProvider，
/// 2xx→原样返回（fix v1.0.3 R6 抽离）
pub(crate) async fn check_status(
    response: reqwest::Response,
    provider_name: &str,
) -> Result<reqwest::Response> {
    let status = response.status();
    match status {
        reqwest::StatusCode::UNAUTHORIZED => Err(EflowError::LlmAuthFailed(provider_name.into())),
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            Err(EflowError::RateLimited(provider_name.into()))
        }
        _ if status.is_success() => Ok(response),
        _ => {
            // 403 (invalid key) / 5xx 等：4xx/5xx 一律按 provider 错处理，
            // 避免把空 content 误当作成功响应（fix v1.0.2：dummy key 在受限网络下会拿到 403）
            let body = response.text().await.unwrap_or_default();
            Err(EflowError::LlmProvider(
                t!(
                    "err_http",
                    msg = format!("status {}: {}", status.as_u16(), body)
                )
                .to_string(),
            ))
        }
    }
}
