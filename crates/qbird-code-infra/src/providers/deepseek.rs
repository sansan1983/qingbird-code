use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{Value, json};

use qbird_code_models::{Message, UsageStats};

use super::{ChatResponse, ProtocolKind, Provider, ProviderKind, RequestConfig};
use crate::config::DeepseekConfig;

pub struct DeepseekProvider {
    config: DeepseekConfig,
}

impl DeepseekProvider {
    pub fn new(config: DeepseekConfig) -> qbird_code_models::Result<Self> {
        Ok(Self { config })
    }

    /// 将统一 Message 转为 OpenAI 格式的消息
    fn to_openai_messages(messages: &[Message]) -> Vec<Value> {
        messages
            .iter()
            .map(|msg| {
                let mut obj = json!({
                    "role": msg.role_str(),
                    "content": msg.content,
                });

                // reasoning_content: DeepSeek 文档推荐直接拼入（API 在非 tool_calls 场景忽略）
                if let Some(ref rc) = msg.reasoning_content {
                    obj["reasoning_content"] = json!(rc);
                }

                // tool_calls
                if let Some(ref tc) = msg.tool_calls {
                    obj["tool_calls"] = json!(tc);
                }

                // tool results
                if let Some(ref tci) = msg.tool_call_id {
                    obj["tool_call_id"] = json!(tci);
                }
                if let Some(ref name) = msg.name {
                    obj["name"] = json!(name);
                }

                obj
            })
            .collect()
    }
}

#[async_trait]
impl Provider for DeepseekProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::DeepSeek
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

        // thinking 参数通过 extra_body 传入（OpenAI SDK 风格）
        body["thinking"] =
            json!({"type": if self.config.thinking_enabled { "enabled" } else { "disabled" }});

        // effort 控制
        if let Some(ref effort) = config.thinking_effort {
            body["reasoning_effort"] = json!(effort);
        }

        // 工具定义
        if !config.tools.is_empty() {
            body["tools"] = json!(config.tools);
        }

        // DeepSeek 思考模式下 temperature 无效（官方文档明确说明）
        if !self.config.thinking_enabled
            && let Some(t) = config.temperature
        {
            body["temperature"] = json!(t);
        }
        if let Some(mt) = config.max_tokens {
            body["max_tokens"] = json!(mt);
        }

        body
    }

    async fn parse_response(&self, body: &Value) -> qbird_code_models::Result<ChatResponse> {
        let choice = body["choices"].get(0).ok_or_else(|| {
            qbird_code_models::EflowError::LlmProvider("No choices in response".into())
        })?;

        let message = &choice["message"];
        let content = message["content"].as_str().unwrap_or("").to_string();
        let reasoning_content = message["reasoning_content"].as_str().map(String::from);
        let finish_reason = choice["finish_reason"].as_str().map(String::from);

        let tool_calls = message["tool_calls"].as_array().cloned();

        // 提取 usage（含缓存统计）
        let usage = body["usage"].clone();
        let usage_stats = UsageStats {
            prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0),
            completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0),
            cache_hit_tokens: usage["prompt_cache_hit_tokens"].as_u64().unwrap_or(0),
            cache_miss_tokens: usage["prompt_cache_miss_tokens"].as_u64().unwrap_or(0),
        };

        Ok(ChatResponse {
            content,
            reasoning_content,
            tool_calls,
            finish_reason,
            usage: usage_stats,
        })
    }

    fn build_headers(&self) -> HashMap<String, String> {
        let api_key = self
            .config
            .api_key
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())
            .unwrap_or_default();
        let mut headers = HashMap::new();
        headers.insert("Authorization".into(), format!("Bearer {}", api_key));
        headers.insert("Content-Type".into(), "application/json".into());
        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qbird_code_models::MessageRole;

    #[test]
    fn test_new_deepseek_provider() {
        let config = DeepseekConfig::default();
        let provider = DeepseekProvider::new(config).unwrap();
        assert_eq!(provider.kind(), ProviderKind::DeepSeek);
        assert_eq!(provider.protocol(), ProtocolKind::OpenAICompatible);
        assert_eq!(provider.model(), "deepseek-v4-pro");
    }

    #[test]
    fn test_to_openai_messages_basic() {
        let messages = vec![
            Message {
                role: MessageRole::System,
                content: "You are a helpful assistant.".into(),
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
        let result = DeepseekProvider::to_openai_messages(&messages);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["role"], "system");
        assert_eq!(result[0]["content"], "You are a helpful assistant.");
        assert_eq!(result[1]["role"], "user");
        assert_eq!(result[1]["content"], "Hello!");
    }

    #[test]
    fn test_to_openai_messages_with_reasoning() {
        let messages = vec![Message {
            role: MessageRole::Assistant,
            content: "Final answer".into(),
            reasoning_content: Some("Step-by-step reasoning...".into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }];
        let result = DeepseekProvider::to_openai_messages(&messages);
        assert_eq!(result[0]["reasoning_content"], "Step-by-step reasoning...");
    }

    #[test]
    fn test_build_request_body_default() {
        let config = DeepseekConfig::default();
        let provider = DeepseekProvider::new(config).unwrap();
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

        assert_eq!(body["model"], "deepseek-v4-pro");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "Hi");
        assert!(!body["stream"].as_bool().unwrap());

        // thinking enabled by default
        assert_eq!(body["thinking"]["type"], "enabled");
    }

    #[test]
    fn test_build_headers_with_env_key() {
        // Without setting env var, it should be empty or fallback
        let config = DeepseekConfig::default();
        let provider = DeepseekProvider::new(config).unwrap();
        let headers = provider.build_headers();
        assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
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
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "prompt_cache_hit_tokens": 5,
                "prompt_cache_miss_tokens": 5
            }
        });

        let config = DeepseekConfig::default();
        let provider = DeepseekProvider::new(config).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.parse_response(&json)).unwrap();

        assert_eq!(result.content, "Hello! How can I help?");
        assert_eq!(result.finish_reason.unwrap(), "stop");
        assert_eq!(result.usage.prompt_tokens, 10);
        assert_eq!(result.usage.completion_tokens, 20);
        assert_eq!(result.usage.cache_hit_tokens, 5);
        assert_eq!(result.usage.cache_miss_tokens, 5);
        assert!(result.reasoning_content.is_none());
    }

    #[test]
    fn test_parse_response_with_reasoning() {
        let json = json!({
            "choices": [{
                "message": {
                    "content": "Final answer",
                    "reasoning_content": "Let me think...",
                    "role": "assistant"
                },
                "finish_reason": "stop",
                "index": 0
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20
            }
        });

        let config = DeepseekConfig::default();
        let provider = DeepseekProvider::new(config).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.parse_response(&json)).unwrap();

        assert_eq!(result.content, "Final answer");
        assert_eq!(result.reasoning_content.unwrap(), "Let me think...");
    }

    #[test]
    fn test_parse_response_no_choices_error() {
        let json = json!({
            "usage": {}
        });

        let config = DeepseekConfig::default();
        let provider = DeepseekProvider::new(config).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.parse_response(&json));

        assert!(result.is_err());
    }

    #[test]
    fn test_build_request_body_with_tools() {
        let config = DeepseekConfig::default();
        let provider = DeepseekProvider::new(config).unwrap();
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
                "description": "Get weather for a location",
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

        // thinking 由 DeepseekConfig 控制，默认开启
        assert_eq!(body["thinking"]["type"], "enabled");
        assert!(body["tools"].is_array());
        assert_eq!(body["tools"].as_array().unwrap().len(), 1);
    }
}
