//! HTTP 客户端基类 - 提取自 generic_anthropic.rs / generic_openai.rs
//!
//! 提供通用 reqwest 客户端、配置管理、URL 构建、HTTP 发送和错误处理等功能。
//! Provider-specific 逻辑通过 LlmProtocol trait 实现。

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::common::error::{EflowError, Result};
use rust_i18n::t;

/// HTTP 客户端配置
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    pub id: String,
    pub api_key: String,
    pub default_model: String,
    pub base_url: String,
    pub timeout_secs: u64,
    pub max_retries: u8,
    pub retry_backoff_ms: u64,
    pub model_endpoints: HashMap<String, String>,
}

/// HTTP 客户端协议 - 提供商特定实现
#[async_trait]
pub trait LlmProtocol: Send + Sync {
    /// 获取默认路径
    fn get_default_path(&self) -> &'static str;

    /// 构建认证头
    fn build_auth_headers(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder;

    /// 解析聊天响应
    fn parse_chat_response(
        &self,
        json: &Value,
    ) -> (
        String,
        Option<Vec<crate::infrastructure::llm::types::ToolCall>>,
        crate::infrastructure::llm::types::TokenUsage,
        String,
    );
}

/// HTTP 客户端基类
pub struct HttpLlmClient<P: LlmProtocol> {
    pub config: HttpClientConfig,
    client: Client,
    protocol: P,
}

impl<P: LlmProtocol> HttpLlmClient<P> {
    /// 创建新的 HTTP 客户端
    pub fn new(config: HttpClientConfig, protocol: P) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| {
                EflowError::Internal(t!("err_llm_init", msg = e.to_string()).to_string())
            })?;

        Ok(Self {
            config,
            client,
            protocol,
        })
    }

    /// 获取重试参数
    pub fn retry_params(&self) -> (u8, u64) {
        (self.config.max_retries, self.config.retry_backoff_ms)
    }

    /// 构建 POST 请求 URL
    fn build_url(&self, model: &str) -> String {
        let path = self
            .config
            .model_endpoints
            .get(model)
            .map(String::as_str)
            .unwrap_or(self.protocol.get_default_path());

        format!("{}{}", self.config.base_url.trim_end_matches('/'), path)
    }

    /// 构建 POST 请求
    pub fn build_post(&self, body: &Value) -> reqwest::RequestBuilder {
        let url = self.build_url(body["model"].as_str().unwrap_or(""));
        self.protocol
            .build_auth_headers(self.client.post(url))
            .json(body)
    }

    /// 发送 HTTP 请求并检查状态
    #[allow(dead_code)]
    async fn send_request(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let response = request.send().await.map_err(|e| {
            EflowError::LlmProvider(t!("err_http", msg = e.to_string()).to_string())
        })?;

        check_status(response, self.config.id.as_str()).await
    }

    /// 解析聊天响应
    pub async fn parse_chat_response(
        &self,
        json: &Value,
    ) -> Result<(
        String,
        Option<Vec<crate::infrastructure::llm::types::ToolCall>>,
        crate::infrastructure::llm::types::TokenUsage,
        String,
    )> {
        let (content, tool_calls, usage, finish_reason) = self.protocol.parse_chat_response(json);
        Ok((content, tool_calls, usage, finish_reason))
    }
}

/// 检查 HTTP 状态码
pub async fn check_status(
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
            let body = response.text().await.unwrap_or_else(|_| "[unreadable error body]".into());
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
