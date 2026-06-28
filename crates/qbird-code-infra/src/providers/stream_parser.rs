use futures_util::StreamExt;
use qbird_code_models::UsageStats;
use serde_json::Value;

use super::ChatResponse;
use super::stream_format::{StreamEvent, ToolCallDelta};

/// Parsed SSE line — either a parsed event or a skip signal.
#[derive(Debug)]
pub enum SseLine {
    /// An event type name (for Anthropic format).
    Event(String),
    /// A data payload (JSON or `[DONE]`).
    Data(String),
    /// Empty or comment line — skip.
    Skip,
}

/// Parse a raw SSE line into an `SseLine`.
pub fn parse_sse_line(line: &str) -> SseLine {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with(':') {
        return SseLine::Skip;
    }
    if let Some(rest) = trimmed.strip_prefix("event: ") {
        return SseLine::Event(rest.trim().to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("data: ") {
        return SseLine::Data(rest.trim().to_string());
    }
    SseLine::Skip
}

// ===== OpenAI-compatible chunk parsing =====

/// Parse a single OpenAI-compatible SSE chunk into a `StreamEvent`.
///
/// OpenAI SSE format:
/// ```json
/// {"choices":[{"delta":{"content":"text"},"index":0}],"usage":{...}}
/// ```
pub fn parse_openai_chunk(chunk: &Value) -> Option<StreamEvent> {
    // Check for [DONE] signal
    if chunk.as_str() == Some("[DONE]") {
        return None;
    }

    let choice = chunk["choices"].get(0)?;
    let delta = &choice["delta"];
    let finish_reason = choice["finish_reason"].as_str();

    // Text content delta
    if let Some(content) = delta["content"].as_str()
        && !content.is_empty()
    {
        return Some(StreamEvent::TextDelta(content.to_string()));
    }

    // Reasoning content delta (DeepSeek extension)
    if let Some(reasoning) = delta["reasoning_content"].as_str()
        && !reasoning.is_empty()
    {
        return Some(StreamEvent::ReasoningDelta(reasoning.to_string()));
    }

    // Tool call deltas
    if let Some(tool_calls) = delta["tool_calls"].as_array()
        && let Some(tc) = tool_calls.first()
    {
        let index = tc["index"].as_u64().unwrap_or(0) as usize;
        let delta = ToolCallDelta {
            id: tc["id"].as_str().map(String::from),
            name: tc["function"]["name"].as_str().map(String::from),
            arguments_delta: tc["function"]["arguments"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        };
        return Some(StreamEvent::ToolCallDelta { index, delta });
    }

    // If finish_reason is present, this is the final chunk — build Done response
    if finish_reason.is_some() {
        let usage = chunk["usage"].clone();
        let resp = ChatResponse {
            content: String::new(), // filled by ReactLoop from accumulated deltas
            reasoning_content: None,
            tool_calls: None,
            finish_reason: finish_reason.map(String::from),
            usage: UsageStats {
                prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0),
                completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0),
                cache_hit_tokens: usage["prompt_cache_hit_tokens"].as_u64().unwrap_or(0),
                cache_miss_tokens: usage["prompt_cache_miss_tokens"].as_u64().unwrap_or(0),
            },
        };
        return Some(StreamEvent::Done(resp));
    }

    None
}

// ===== Anthropic chunk parsing =====

/// State accumulated across Anthropic SSE events to produce a final ChatResponse.
#[derive(Debug, Default)]
pub struct AnthropicStreamState {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub stop_reason: Option<String>,
}

/// Parse a single Anthropic SSE event into a `StreamEvent`.
///
/// Anthropic SSE event types:
/// - `message_start` → usage (input_tokens)
/// - `content_block_delta` → text or tool_use delta
/// - `message_delta` → usage (output_tokens) + stop_reason
/// - `message_stop` → stream done
pub fn parse_anthropic_event(
    event_type: &str,
    data: &Value,
    state: &mut AnthropicStreamState,
) -> Option<StreamEvent> {
    match event_type {
        "message_start" => {
            if let Some(usage) = data.get("message").and_then(|m| m.get("usage")) {
                state.input_tokens = usage["input_tokens"].as_u64().unwrap_or(0);
                state.cache_read_input_tokens =
                    usage["cache_read_input_tokens"].as_u64().unwrap_or(0);
            }
            None
        }
        "content_block_delta" => {
            let delta = data.get("delta")?;
            match delta["type"].as_str() {
                Some("text_delta") => {
                    let text = delta["text"].as_str().unwrap_or("");
                    if text.is_empty() {
                        None
                    } else {
                        Some(StreamEvent::TextDelta(text.to_string()))
                    }
                }
                Some("thinking_delta") => {
                    let thinking = delta["thinking"].as_str().unwrap_or("");
                    if !thinking.is_empty() {
                        return Some(StreamEvent::ReasoningDelta(thinking.to_string()));
                    }
                    None
                }
                Some("input_json_delta") => {
                    let args = delta["partial_json"].as_str().unwrap_or("");
                    let index = data["index"].as_u64().unwrap_or(0) as usize;
                    Some(StreamEvent::ToolCallDelta {
                        index,
                        delta: ToolCallDelta {
                            id: None,
                            name: None,
                            arguments_delta: args.to_string(),
                        },
                    })
                }
                _ => None,
            }
        }
        "content_block_start" => {
            let block = data.get("content_block")?;
            if block["type"].as_str() == Some("tool_use") {
                let index = data["index"].as_u64().unwrap_or(0) as usize;
                return Some(StreamEvent::ToolCallDelta {
                    index,
                    delta: ToolCallDelta {
                        id: block["id"].as_str().map(String::from),
                        name: block["name"].as_str().map(String::from),
                        arguments_delta: String::new(),
                    },
                });
            }
            None
        }
        "message_delta" => {
            let delta = data.get("delta")?;
            state.stop_reason = delta["stop_reason"].as_str().map(String::from);
            if let Some(usage) = data.get("usage") {
                state.output_tokens = usage["output_tokens"].as_u64().unwrap_or(0);
            }
            None
        }
        "message_stop" => {
            let resp = ChatResponse {
                content: String::new(), // filled by ReactLoop from accumulated deltas
                reasoning_content: None,
                tool_calls: None,
                finish_reason: state.stop_reason.clone(),
                usage: UsageStats {
                    prompt_tokens: state.input_tokens,
                    completion_tokens: state.output_tokens,
                    cache_hit_tokens: state.cache_read_input_tokens,
                    cache_miss_tokens: 0,
                },
            };
            Some(StreamEvent::Done(resp))
        }
        _ => None,
    }
}

// ===== Provider-level streaming helpers =====

use tokio::sync::mpsc;

/// Run an OpenAI-compatible SSE streaming loop. Reads lines from the
/// response, parses each JSON chunk, and sends `StreamEvent`s to `tx`.
/// Returns when the stream ends or `[DONE]` is received.
pub async fn run_openai_stream(resp: reqwest::Response, tx: mpsc::Sender<StreamEvent>) {
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_result) = stream.next().await {
        let bytes = match chunk_result {
            Ok(b) => b,
            Err(e) => {
                let _ = tx.send(StreamEvent::Error(e.to_string())).await;
                return;
            }
        };

        buffer.push_str(&String::from_utf8_lossy(&bytes));

        // Process complete lines
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim_end_matches('\r').to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            // Strip "data: " prefix
            let data_str = match parse_sse_line(&line) {
                SseLine::Data(s) => s,
                _ => continue,
            };

            // [DONE] signal
            if data_str == "[DONE]" {
                return;
            }

            // Parse JSON chunk
            match serde_json::from_str::<Value>(&data_str) {
                Ok(chunk) => {
                    if let Some(event) = parse_openai_chunk(&chunk) {
                        let is_done = matches!(event, StreamEvent::Done(_));
                        if tx.send(event).await.is_err() {
                            return; // receiver dropped
                        }
                        if is_done {
                            return;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(StreamEvent::Error(format!("JSON parse error: {e}")))
                        .await;
                }
            }
        }
    }
}

/// Run an Anthropic SSE streaming loop. Reads lines from the response,
/// parses event type + data, and sends `StreamEvent`s to `tx`.
pub async fn run_anthropic_stream(resp: reqwest::Response, tx: mpsc::Sender<StreamEvent>) {
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    let mut state = AnthropicStreamState::default();
    let mut current_event_type = String::new();

    while let Some(chunk_result) = stream.next().await {
        let bytes = match chunk_result {
            Ok(b) => b,
            Err(e) => {
                let _ = tx.send(StreamEvent::Error(e.to_string())).await;
                return;
            }
        };

        buffer.push_str(&String::from_utf8_lossy(&bytes));

        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim_end_matches('\r').to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if line.is_empty() {
                // Empty line = end of event block, reset event type
                current_event_type.clear();
                continue;
            }

            match parse_sse_line(&line) {
                SseLine::Event(ev) => {
                    current_event_type = ev;
                }
                SseLine::Data(data_str) => match serde_json::from_str::<Value>(&data_str) {
                    Ok(data) => {
                        if let Some(event) =
                            parse_anthropic_event(&current_event_type, &data, &mut state)
                        {
                            let is_done = matches!(event, StreamEvent::Done(_));
                            if tx.send(event).await.is_err() {
                                return;
                            }
                            if is_done {
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(StreamEvent::Error(format!("JSON parse error: {e}")))
                            .await;
                    }
                },
                SseLine::Skip => {}
            }
        }
    }
}
