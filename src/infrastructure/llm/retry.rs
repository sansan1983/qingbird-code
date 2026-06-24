//! Retry logic — 指数退避
//!
//! V0.1.0: 基础指数退避实现

use std::time::Duration;

use super::tier::TierRouter;
use crate::common::error::{EflowError, Result};
use crate::common::types::ModelTier;
use crate::infrastructure::llm::types::ChatRequest;

/// 指数退避重试（最多 max_retries 次，间隔 backoff_ms * 2^n）
///
/// 对 `RateLimited` 和 `LlmProvider`（含 4xx/5xx）两类瞬时错误都重试。
pub async fn chat_with_retry(
    router: &TierRouter,
    tier: ModelTier,
    request: ChatRequest,
    max_retries: u8,
    backoff_ms: u64,
) -> Result<super::types::ChatResponse> {
    let mut attempt = 0u8;
    loop {
        match router.chat(tier, request.clone()).await {
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
