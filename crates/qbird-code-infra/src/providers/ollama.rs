use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{Value, json};

use qbird_code_models::{Message, UsageStats};

use super::{ChatResponse, ProtocolKind, Provider, ProviderKind, RequestConfig, StreamEvent};
use crate::config::OllamaConfig;

pub struct OllamaProvider {
    config: OllamaConfig,
}

impl OllamaProvider {
    pub fn new(config: OllamaConfig) -> qbird_code_models::Result<Self> {
        Ok(Self { config })
    }

    fn to_openai_messages(messages: &[Message]) -> Vec<Value> {
        messages
            .iter()
            .map(|msg| {
                json!({
                    "role": msg.role_str(),
                    "content": msg.content,
                })
            })
            .collect()
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ollama
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
        let mut body = json!({
            "model": model,
            "messages": Self::to_openai_messages(messages),
            "stream": config.stream,
        });

        if !config.tools.is_empty() {
            body["tools"] = json!(config.tools);
        }
        if let Some(t) = config.temperature {
            body["temperature"] = json!(t);
        }
        if let Some(mt) = config.max_tokens {
            body["max_tokens"] = json!(mt);
        }
        body
    }

    async fn parse_response(&self, body: &Value) -> qbird_code_models::Result<ChatResponse> {
        let choice = body["choices"]
            .get(0)
            .ok_or_else(|| qbird_code_models::EflowError::LlmProvider("No choices".into()))?;
        let message = &choice["message"];
        let usage = body["usage"].clone();
        Ok(ChatResponse {
            content: message["content"].as_str().unwrap_or("").to_string(),
            reasoning_content: None,
            tool_calls: message["tool_calls"].as_array().cloned(),
            finish_reason: choice["finish_reason"].as_str().map(String::from),
            usage: UsageStats {
                prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0),
                completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0),
                ..Default::default()
            },
        })
    }

    fn build_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
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

    #[test]
    fn test_new_ollama_provider() {
        let config = OllamaConfig::default();
        let provider = OllamaProvider::new(config).unwrap();
        assert_eq!(provider.kind(), ProviderKind::Ollama);
        assert_eq!(provider.protocol(), ProtocolKind::OpenAICompatible);
        assert_eq!(provider.model(), "qwen2.5:14b");
    }

    #[test]
    fn test_to_openai_messages() {
        let messages = vec![
            Message {
                role: MessageRole::System,
                content: "You are helpful.".into(),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
            },
            Message {
                role: MessageRole::User,
                content: "Hello!".into(),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
            },
        ];
        let result = OllamaProvider::to_openai_messages(&messages);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["role"], "system");
        assert_eq!(result[0]["content"], "You are helpful.");
        assert_eq!(result[1]["role"], "user");
        assert_eq!(result[1]["content"], "Hello!");
    }

    #[test]
    fn test_build_request_body_default() {
        let config = OllamaConfig::default();
        let provider = OllamaProvider::new(config).unwrap();
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

        assert_eq!(body["model"], "qwen2.5:14b");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "Hi");
        assert_eq!(body["stream"], false);

        // Ollama 无 thinking 参数
        assert!(body.get("thinking").is_none());
    }

    #[test]
    fn test_build_request_body_with_tools() {
        let config = OllamaConfig::default();
        let provider = OllamaProvider::new(config).unwrap();
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
            temperature: None,
            max_tokens: None,
            stream: false,
            thinking_enabled: false,
            thinking_effort: None,
            tools: vec![tool_def],
        };
        let body = provider.build_request_body(&messages, &req_cfg);
        assert!(body["tools"].is_array());
        assert_eq!(body["tools"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_build_headers_no_authorization() {
        let config = OllamaConfig::default();
        let provider = OllamaProvider::new(config).unwrap();
        let headers = provider.build_headers();
        assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
        // Ollama 不需要 Authorization
        assert!(!headers.contains_key("Authorization"));
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

        let config = OllamaConfig::default();
        let provider = OllamaProvider::new(config).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.parse_response(&json)).unwrap();

        assert_eq!(result.content, "Hello! How can I help?");
        assert_eq!(result.finish_reason.unwrap(), "stop");
        assert!(result.reasoning_content.is_none());
        assert!(result.tool_calls.is_none());
    }

    #[test]
    fn test_parse_response_no_choices_error() {
        let json = json!({ "error": "no choices" });
        let config = OllamaConfig::default();
        let provider = OllamaProvider::new(config).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.parse_response(&json));
        assert!(result.is_err());
    }

    #[test]
    fn test_build_request_body_temperature_and_max_tokens() {
        let config = OllamaConfig::default();
        let provider = OllamaProvider::new(config).unwrap();
        let messages = vec![Message {
            role: MessageRole::User,
            content: "Hi".into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }];
        let req_cfg = RequestConfig {
            model: String::new(),
            temperature: Some(0.5),
            max_tokens: Some(2048),
            stream: false,
            thinking_enabled: false,
            thinking_effort: None,
            tools: vec![],
        };
        let body = provider.build_request_body(&messages, &req_cfg);
        assert_eq!(body["temperature"], 0.5);
        assert_eq!(body["max_tokens"], 2048);
    }
}
