use std::time::Duration;

use qbird_code_models::{Result, RetryPolicy};
use reqwest::Client;
use serde_json::Value;

use crate::providers::Provider;

/// 统一 HTTP 客户端 — 支持 OpenAI 和 Anthropic 两种协议
pub struct HttpLlmClient {
    client: Client,
    /// Retry policy (max retries, exponential backoff with max cap).
    /// Defaults to `RetryPolicy::default()` (3 retries, 1s → 2s → 4s, capped at 30s).
    retry_policy: RetryPolicy,
}

impl HttpLlmClient {
    pub fn new(timeout_secs: u64, retry_policy: RetryPolicy) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| qbird_code_models::EflowError::LlmProvider(e.to_string()))?;

        Ok(Self {
            client,
            retry_policy,
        })
    }

    /// 发送请求并解析响应（带重试）
    pub async fn send(&self, provider: &dyn Provider, body: &Value) -> Result<Value> {
        let endpoint = provider.endpoint();
        let headers = provider.build_headers();

        let mut last_error = String::new();

        for attempt in 0..=self.retry_policy.max_retries {
            let mut req = self.client.post(&endpoint).json(body);

            for (k, v) in &headers {
                req = req.header(k.as_str(), v.as_str());
            }

            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();

                    // 401/403 — 鉴权失败，不重试
                    if status == 401 || status == 403 {
                        return Err(qbird_code_models::EflowError::LlmAuthFailed(format!(
                            "Auth failed: {}",
                            status
                        )));
                    }

                    // 400 — 请求格式错误，不重试（除非是 reasoning_content 相关）
                    if status == 400 {
                        let body_text = resp.text().await.unwrap_or_default();
                        return Err(qbird_code_models::EflowError::LlmProvider(format!(
                            "Bad request: {}",
                            body_text
                        )));
                    }

                    // 429 — 频率限制，等待后重试
                    if status == 429 {
                        let retry_after = resp
                            .headers()
                            .get("retry-after")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(self.retry_policy.initial_backoff_ms / 1000);
                        tokio::time::sleep(Duration::from_secs(retry_after)).await;
                        continue;
                    }

                    // 成功或 5xx
                    if status.is_success() {
                        let json: Value = resp.json().await.map_err(|e| {
                            qbird_code_models::EflowError::LlmProvider(format!("Parse error: {e}"))
                        })?;
                        return Ok(json);
                    }

                    last_error = format!("HTTP {}", status);
                }
                Err(e) => {
                    last_error = e.to_string();
                }
            }

            if attempt < self.retry_policy.max_retries {
                let backoff = self.retry_policy.backoff_for_attempt(attempt);
                tokio::time::sleep(Duration::from_millis(backoff)).await;
            }
        }

        Err(qbird_code_models::EflowError::LlmProvider(last_error))
    }
}
