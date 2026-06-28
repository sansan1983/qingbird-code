use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use qbird_code_models::{Message, UsageStats};

use super::{ChatResponse, ProtocolKind, Provider, ProviderKind, RequestConfig, StreamEvent};
use crate::config::OpenaiConfig;

pub struct OpenAIProvider {
    config: OpenaiConfig,
}

impl OpenAIProvider {
    pub fn new(config: OpenaiConfig) -> qbird_code_models::Result<Self> {
        Ok(Self { config })
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAI
    }
    fn protocol(&self) -> ProtocolKind {
        ProtocolKind::OpenAICompatible
    }
    fn model(&self) -> &str {
        &self.config.default_model
    }
    fn base_url(&self) -> &str {
        &self.config.base_url
    }

    fn build_request_body(&self, messages: &[Message], config: &RequestConfig) -> Value {
        let model = if config.model.is_empty() {
            self.config.default_model.clone()
        } else {
            config.model.clone()
        };
        let mut body = serde_json::json!({
            "model": model,
            "messages": messages.iter().map(|m| serde_json::json!({
                "role": m.role_str(),
                "content": m.content,
            })).collect::<Vec<_>>(),
            "stream": config.stream,
        });
        if !config.tools.is_empty() {
            body["tools"] = serde_json::json!(config.tools);
        }
        if let Some(t) = config.temperature {
            body["temperature"] = serde_json::json!(t);
        }
        if let Some(mt) = config.max_tokens {
            body["max_tokens"] = serde_json::json!(mt);
        }
        body
    }

    async fn parse_response(&self, body: &Value) -> qbird_code_models::Result<ChatResponse> {
        let choice = body["choices"]
            .get(0)
            .ok_or_else(|| qbird_code_models::EflowError::LlmProvider("No choices".into()))?;
        let msg = &choice["message"];
        let usage = body["usage"].clone();
        Ok(ChatResponse {
            content: msg["content"].as_str().unwrap_or("").to_string(),
            reasoning_content: None,
            tool_calls: msg["tool_calls"].as_array().cloned(),
            finish_reason: choice["finish_reason"].as_str().map(String::from),
            usage: UsageStats {
                prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0),
                completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0),
                ..Default::default()
            },
        })
    }

    fn build_headers(&self) -> HashMap<String, String> {
        let key = self
            .config
            .api_key
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .unwrap_or_default();
        let mut headers = HashMap::new();
        headers.insert("Authorization".into(), format!("Bearer {}", key));
        headers.insert("Content-Type".into(), "application/json".into());
        headers
    }

    async fn stream(
        &self,
        http_client: &crate::http_client::HttpLlmClient,
        messages: &[Message],
        config: &RequestConfig,
    ) -> qbird_code_models::Result<tokio::sync::mpsc::Receiver<StreamEvent>> {
        let mut req_config = config.clone();
        req_config.stream = true;
        let body = self.build_request_body(messages, &req_config);
        let resp = http_client.send_streaming(self, &body).await?;
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        tokio::spawn(async move {
            super::stream_parser::run_openai_stream(resp, tx).await;
        });
        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qbird_code_models::MessageRole;
    use serde_json::json;

    #[test]
    fn test_new_openai_provider() {
        let config = OpenaiConfig::default();
        let provider = OpenAIProvider::new(config).unwrap();
        assert_eq!(provider.kind(), ProviderKind::OpenAI);
        assert_eq!(provider.protocol(), ProtocolKind::OpenAICompatible);
        assert_eq!(provider.model(), "gpt-4o");
    }

    #[test]
    fn test_build_request_body_default() {
        let config = OpenaiConfig::default();
        let provider = OpenAIProvider::new(config).unwrap();
        let messages = vec![Message {
            role: MessageRole::User,
            content: "Hi".into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }];
        let req_cfg = RequestConfig::default();
        let body = provider.build_request_body(&messages, &req_cfg);

        assert_eq!(body["model"], "gpt-4o");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "Hi");
        assert!(!body["stream"].as_bool().unwrap());

        // OpenAI placeholder has no thinking params
        assert!(body.get("thinking").is_none());
    }

    #[test]
    fn test_build_request_body_with_tools() {
        let config = OpenaiConfig::default();
        let provider = OpenAIProvider::new(config).unwrap();
        let messages = vec![Message {
            role: MessageRole::User,
            content: "What's the weather?".into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }];
        let tool_def = json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get weather",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    },
                    "required": ["location"]
                }
            }
        });
        let req_cfg = RequestConfig {
            model: String::new(),
            temperature: Some(0.5),
            max_tokens: Some(2048),
            stream: false,
            thinking_enabled: false,
            thinking_effort: None,
            tools: vec![tool_def],
        };
        let body = provider.build_request_body(&messages, &req_cfg);
        assert_eq!(body["temperature"], 0.5);
        assert_eq!(body["max_tokens"], 2048);
        assert!(body["tools"].is_array());
        assert_eq!(body["tools"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_build_headers_with_env_key_fallback() {
        let config = OpenaiConfig::default();
        let provider = OpenAIProvider::new(config).unwrap();
        let headers = provider.build_headers();
        assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
        // May be empty if no env var set, but Bearer format
        assert!(headers.contains_key("Authorization"));
    }

    #[test]
    fn test_parse_response_basic() {
        let json = json!({
            "choices": [{
                "message": {
                    "content": "Hello! How can I help?",
                    "role": "assistant"
                },
                "finish_reason": "stop",
                "index": 0
            }]
        });

        let config = OpenaiConfig::default();
        let provider = OpenAIProvider::new(config).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.parse_response(&json)).unwrap();

        assert_eq!(result.content, "Hello! How can I help?");
        assert_eq!(result.finish_reason.unwrap(), "stop");
        assert!(result.reasoning_content.is_none());
    }

    #[test]
    fn test_parse_response_no_choices_error() {
        let json = json!({ "error": "no choices" });
        let config = OpenaiConfig::default();
        let provider = OpenAIProvider::new(config).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.parse_response(&json));
        assert!(result.is_err());
    }
}
