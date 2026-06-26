use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{Value, json};

use qbird_code_models::{Message, MessageRole, UsageStats};

use super::{ChatResponse, ProtocolKind, Provider, ProviderKind, RequestConfig};
use crate::config::DeepseekConfig;
use crate::http_client::HttpLlmClient;

pub struct DeepseekAnthropicProvider {
    config: DeepseekConfig,
    #[allow(dead_code)]
    http: HttpLlmClient,
}

impl DeepseekAnthropicProvider {
    pub fn new(config: DeepseekConfig) -> qbird_code_models::Result<Self> {
        let http = HttpLlmClient::new(
            config.timeout_secs,
            config.max_retries,
            config.retry_backoff_ms,
        )?;
        Ok(Self { config, http })
    }

    /// 将工具定义从 OpenAI 格式转为 Anthropic 格式
    fn convert_tools(openai_tools: &[Value]) -> Vec<Value> {
        openai_tools
            .iter()
            .filter_map(|t| {
                let func = t.get("function")?;
                Some(json!({
                    "name": func["name"],
                    "description": func.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                    "input_schema": func["parameters"].clone(),
                }))
            })
            .collect()
    }

    /// 将 tool_calls 从 Anthropic 格式转回统一格式
    fn parse_anthropic_tool_calls(content_blocks: &[Value]) -> Option<Vec<Value>> {
        let calls: Vec<Value> = content_blocks
            .iter()
            .filter(|b| b["type"].as_str() == Some("tool_use"))
            .map(|b| {
                json!({
                    "id": b["id"],
                    "type": "function",
                    "function": {
                        "name": b["name"],
                        "arguments": serde_json::to_string(&b["input"]).unwrap_or_default(),
                    }
                })
            })
            .collect();
        if calls.is_empty() { None } else { Some(calls) }
    }
}

#[async_trait]
#[allow(clippy::misnamed_getters)]
impl Provider for DeepseekAnthropicProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::DeepSeek
    }
    fn protocol(&self) -> ProtocolKind {
        ProtocolKind::Anthropic
    }
    fn model(&self) -> &str {
        &self.config.default_model
    }
    fn base_url(&self) -> &str {
        &self.config.base_url_anthropic
    }

    fn build_request_body(&self, messages: &[Message], config: &RequestConfig) -> Value {
        // Anthropic 格式：system 单独字段，messages 不含 system role
        let system_content: String = messages
            .iter()
            .filter(|m| m.role == MessageRole::System)
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        let anthropic_msgs: Vec<Value> = messages.iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| {
                let mut content_blocks: Vec<Value> = vec![];

                if !m.content.is_empty() {
                    content_blocks.push(json!({"type": "text", "text": m.content}));
                }

                // reasoning_content as thinking block
                if let Some(ref rc) = m.reasoning_content {
                    content_blocks.push(json!({
                        "type": "thinking",
                        "thinking": rc,
                    }));
                }

                // tool_calls as tool_use blocks
                if let Some(ref tc) = m.tool_calls {
                    for call in tc {
                        content_blocks.push(json!({
                            "type": "tool_use",
                            "id": call.id,
                            "name": call.function.name,
                            "input": serde_json::from_str::<Value>(&call.function.arguments).unwrap_or(json!({})),
                        }));
                    }
                }

                // tool results as tool_result blocks
                if m.role == MessageRole::Tool {
                    content_blocks = vec![json!({
                        "type": "tool_result",
                        "tool_use_id": m.tool_call_id,
                        "content": m.content,
                    })];
                }

                json!({
                    "role": match m.role {
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        _ => "user",  // Tool messages are user in Anthropic format
                    },
                    "content": content_blocks,
                })
            })
            .collect();

        let mut body = json!({
            "model": self.config.default_model,
            "max_tokens": config.max_tokens.unwrap_or(4096),
            "messages": anthropic_msgs,
        });

        if !system_content.is_empty() {
            body["system"] = json!(system_content);
        }

        if config.thinking_enabled {
            body["thinking"] = json!({"type": "enabled"});
        }

        if let Some(ref effort) = config.thinking_effort {
            body["output_config"] = json!({"effort": effort});
        }

        if !config.tools.is_empty() {
            body["tools"] = json!(Self::convert_tools(&config.tools));
        }

        if let Some(t) = config.temperature {
            body["temperature"] = json!(t);
        }

        body
    }

    async fn parse_response(&self, body: &Value) -> qbird_code_models::Result<ChatResponse> {
        let content_blocks = body["content"].as_array().ok_or_else(|| {
            qbird_code_models::EflowError::LlmProvider("No content in Anthropic response".into())
        })?;

        let mut text = String::new();
        let mut reasoning = String::new();

        for block in content_blocks {
            match block["type"].as_str() {
                Some("text") => {
                    if let Some(t) = block["text"].as_str() {
                        text.push_str(t);
                    }
                }
                Some("thinking") => {
                    if let Some(t) = block["thinking"].as_str() {
                        reasoning.push_str(t);
                    }
                }
                _ => {}
            }
        }

        // Anthropic tool_use blocks → OpenAI 兼容格式
        let tool_calls = Self::parse_anthropic_tool_calls(content_blocks);

        let stop_reason = body["stop_reason"].as_str().unwrap_or("").to_string();

        let usage_stats = UsageStats {
            prompt_tokens: body["usage"]["input_tokens"].as_u64().unwrap_or(0),
            completion_tokens: body["usage"]["output_tokens"].as_u64().unwrap_or(0),
            cache_hit_tokens: body["usage"]["cache_read_input_tokens"]
                .as_u64()
                .unwrap_or(0),
            cache_miss_tokens: 0,
        };

        Ok(ChatResponse {
            content: text,
            reasoning_content: if reasoning.is_empty() {
                None
            } else {
                Some(reasoning)
            },
            tool_calls,
            finish_reason: Some(stop_reason),
            usage: usage_stats,
        })
    }

    fn build_headers(&self) -> HashMap<String, String> {
        let api_key = self
            .config
            .api_key
            .clone()
            .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())
            .unwrap_or_default();
        let mut headers = HashMap::new();
        headers.insert("x-api-key".into(), api_key);
        headers.insert("Content-Type".into(), "application/json".into());
        // Anthropic 版本头（DeepSeek 会忽略但保持兼容）
        headers.insert("anthropic-version".into(), "2023-06-01".into());
        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qbird_code_models::ToolCall;

    #[test]
    fn test_new_anthropic_provider() {
        let config = DeepseekConfig::default();
        let provider = DeepseekAnthropicProvider::new(config).unwrap();
        assert_eq!(provider.kind(), ProviderKind::DeepSeek);
        assert_eq!(provider.protocol(), ProtocolKind::Anthropic);
        assert_eq!(provider.model(), "deepseek-v4-pro");
        assert_eq!(provider.base_url(), "https://api.deepseek.com/anthropic");
    }

    #[test]
    fn test_build_request_body_basic() {
        let config = DeepseekConfig::default();
        let provider = DeepseekAnthropicProvider::new(config).unwrap();
        let messages = vec![Message {
            role: MessageRole::User,
            content: "Hello!".into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }];
        let req_cfg = RequestConfig::default();
        let body = provider.build_request_body(&messages, &req_cfg);

        assert_eq!(body["model"], "deepseek-v4-pro");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"][0]["type"], "text");
        assert_eq!(body["messages"][0]["content"][0]["text"], "Hello!");
        assert!(body["max_tokens"].is_number());
    }

    #[test]
    fn test_build_request_body_with_system() {
        let config = DeepseekConfig::default();
        let provider = DeepseekAnthropicProvider::new(config).unwrap();
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
                content: "Hi!".into(),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
            },
        ];
        let req_cfg = RequestConfig::default();
        let body = provider.build_request_body(&messages, &req_cfg);

        // System message should be in top-level "system" field, not in messages
        assert_eq!(body["system"], "You are a helpful assistant.");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["messages"][0]["role"], "user");
    }

    #[test]
    fn test_build_request_body_with_thinking() {
        let config = DeepseekConfig::default();
        let provider = DeepseekAnthropicProvider::new(config).unwrap();
        let messages = vec![Message {
            role: MessageRole::User,
            content: "Think step by step".into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }];
        let req_cfg = RequestConfig {
            temperature: None,
            max_tokens: Some(8192),
            stream: false,
            thinking_enabled: true,
            thinking_effort: Some("max".into()),
            tools: vec![],
        };
        let body = provider.build_request_body(&messages, &req_cfg);

        assert_eq!(body["thinking"]["type"], "enabled");
        assert_eq!(body["output_config"]["effort"], "max");
        assert_eq!(body["max_tokens"], 8192);
    }

    #[test]
    fn test_build_headers() {
        let config = DeepseekConfig::default();
        let provider = DeepseekAnthropicProvider::new(config).unwrap();
        let headers = provider.build_headers();

        assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
        assert!(headers.contains_key("x-api-key"));
        assert_eq!(headers.get("anthropic-version").unwrap(), "2023-06-01");
        // Should NOT have Authorization header (Anthropic uses x-api-key)
        assert!(!headers.contains_key("Authorization"));
    }

    #[test]
    fn test_parse_response_basic() {
        let json = json!({
            "content": [
                {"type": "text", "text": "Hello! How can I help?"}
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20,
                "cache_read_input_tokens": 5
            }
        });

        let config = DeepseekConfig::default();
        let provider = DeepseekAnthropicProvider::new(config).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.parse_response(&json)).unwrap();

        assert_eq!(result.content, "Hello! How can I help?");
        assert_eq!(result.finish_reason.unwrap(), "end_turn");
        assert_eq!(result.usage.prompt_tokens, 10);
        assert_eq!(result.usage.completion_tokens, 20);
        assert_eq!(result.usage.cache_hit_tokens, 5);
        assert!(result.reasoning_content.is_none());
    }

    #[test]
    fn test_parse_response_with_thinking() {
        let json = json!({
            "content": [
                {"type": "thinking", "thinking": "Let me think step by step..."},
                {"type": "text", "text": "Here is the final answer."}
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 50
            }
        });

        let config = DeepseekConfig::default();
        let provider = DeepseekAnthropicProvider::new(config).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.parse_response(&json)).unwrap();

        assert_eq!(result.content, "Here is the final answer.");
        assert_eq!(
            result.reasoning_content.unwrap(),
            "Let me think step by step..."
        );
    }

    #[test]
    fn test_parse_response_no_content_error() {
        let json = json!({
            "stop_reason": "end_turn",
            "usage": {}
        });

        let config = DeepseekConfig::default();
        let provider = DeepseekAnthropicProvider::new(config).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.parse_response(&json));

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_response_with_tool_calls() {
        let json = json!({
            "content": [
                {
                    "type": "text",
                    "text": "Let me check the weather."
                },
                {
                    "type": "tool_use",
                    "id": "toolu_123",
                    "name": "get_weather",
                    "input": {"location": "Beijing"}
                }
            ],
            "stop_reason": "tool_use",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 30
            }
        });

        let config = DeepseekConfig::default();
        let provider = DeepseekAnthropicProvider::new(config).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.parse_response(&json)).unwrap();

        assert_eq!(result.content, "Let me check the weather.");
        assert!(result.tool_calls.is_some());
        let calls = result.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["function"]["name"], "get_weather");
        assert_eq!(calls[0]["id"], "toolu_123");
    }

    #[test]
    fn test_build_request_body_with_reasoning_and_tool_calls() {
        let config = DeepseekConfig::default();
        let provider = DeepseekAnthropicProvider::new(config).unwrap();
        let messages = vec![Message {
            role: MessageRole::Assistant,
            content: "Final answer".into(),
            reasoning_content: Some("Step-by-step reasoning...".into()),
            tool_calls: Some(vec![ToolCall {
                id: "call_123".into(),
                function: qbird_code_models::ToolCallFunction {
                    name: "get_weather".into(),
                    arguments: r#"{"location": "Beijing"}"#.into(),
                },
            }]),
            tool_call_id: None,
            name: None,
        }];
        let req_cfg = RequestConfig::default();
        let body = provider.build_request_body(&messages, &req_cfg);

        let content = body["messages"][0]["content"].as_array().unwrap();
        // Should have text + thinking + tool_use blocks
        assert_eq!(content.len(), 3);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[1]["type"], "thinking");
        assert_eq!(content[1]["thinking"], "Step-by-step reasoning...");
        assert_eq!(content[2]["type"], "tool_use");
        assert_eq!(content[2]["name"], "get_weather");
    }

    #[test]
    fn test_convert_tools() {
        let openai_tools = vec![json!({
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
        })];

        let anthropic_tools = DeepseekAnthropicProvider::convert_tools(&openai_tools);
        assert_eq!(anthropic_tools.len(), 1);
        assert_eq!(anthropic_tools[0]["name"], "get_weather");
        assert_eq!(
            anthropic_tools[0]["description"],
            "Get weather for a location"
        );
        assert_eq!(anthropic_tools[0]["input_schema"]["type"], "object");
    }
}
