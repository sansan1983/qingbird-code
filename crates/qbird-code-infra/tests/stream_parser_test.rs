use serde_json::json;

use qbird_code_infra::providers::stream_format::{StreamEvent, StreamFormat};
use qbird_code_infra::providers::stream_parser::{
    AnthropicStreamState, SseLine, parse_anthropic_event, parse_openai_chunk, parse_sse_line,
    run_openai_stream,
};

// ===== OpenAI parser (5 tests) =====

#[test]
fn test_openai_text_delta() {
    let chunk = json!({
        "choices": [{
            "delta": {"content": "Hello"},
            "index": 0
        }]
    });
    let event = parse_openai_chunk(&chunk).unwrap();
    match event {
        StreamEvent::TextDelta(s) => assert_eq!(s, "Hello"),
        _ => panic!("expected TextDelta"),
    }
}

#[test]
fn test_openai_reasoning_delta() {
    let chunk = json!({
        "choices": [{
            "delta": {"reasoning_content": "Let me think..."},
            "index": 0
        }]
    });
    let event = parse_openai_chunk(&chunk).unwrap();
    match event {
        StreamEvent::ReasoningDelta(s) => assert_eq!(s, "Let me think..."),
        _ => panic!("expected ReasoningDelta"),
    }
}

#[test]
fn test_openai_tool_call_incremental() {
    let chunk = json!({
        "choices": [{
            "delta": {
                "tool_calls": [{
                    "index": 0,
                    "id": "call_123",
                    "function": {"name": "read_file", "arguments": ""}
                }]
            },
            "index": 0
        }]
    });
    let event = parse_openai_chunk(&chunk).unwrap();
    match event {
        StreamEvent::ToolCallDelta { index, delta } => {
            assert_eq!(index, 0);
            assert_eq!(delta.id.unwrap(), "call_123");
            assert_eq!(delta.name.unwrap(), "read_file");
            assert!(delta.arguments_delta.is_empty());
        }
        _ => panic!("expected ToolCallDelta"),
    }
}

#[test]
fn test_openai_usage_at_end() {
    let chunk = json!({
        "choices": [{
            "delta": {},
            "finish_reason": "stop",
            "index": 0
        }],
        "usage": {
            "prompt_tokens": 100,
            "completion_tokens": 50,
            "prompt_cache_hit_tokens": 20
        }
    });
    let event = parse_openai_chunk(&chunk).unwrap();
    match event {
        StreamEvent::Done(resp) => {
            assert_eq!(resp.finish_reason.unwrap(), "stop");
            assert_eq!(resp.usage.prompt_tokens, 100);
            assert_eq!(resp.usage.completion_tokens, 50);
            assert_eq!(resp.usage.cache_hit_tokens, 20);
        }
        _ => panic!("expected Done"),
    }
}

#[test]
fn test_openai_empty_delta_returns_none() {
    let chunk = json!({
        "choices": [{
            "delta": {},
            "index": 0
        }]
    });
    let event = parse_openai_chunk(&chunk);
    assert!(event.is_none());
}

// ===== Anthropic parser (5 tests) =====

#[test]
fn test_anthropic_message_start_usage() {
    let data = json!({
        "message": {
            "usage": {
                "input_tokens": 200,
                "cache_read_input_tokens": 50
            }
        }
    });
    let mut state = AnthropicStreamState::default();
    let event = parse_anthropic_event("message_start", &data, &mut state);
    // message_start returns no events, just updates state
    assert!(event.is_none());
    assert_eq!(state.input_tokens, 200);
    assert_eq!(state.cache_read_input_tokens, 50);
}

#[test]
fn test_anthropic_text_delta() {
    let data = json!({
        "index": 0,
        "delta": {"type": "text_delta", "text": "world"}
    });
    let mut state = AnthropicStreamState::default();
    let event = parse_anthropic_event("content_block_delta", &data, &mut state).unwrap();
    match event {
        StreamEvent::TextDelta(s) => assert_eq!(s, "world"),
        _ => panic!("expected TextDelta"),
    }
}

#[test]
fn test_anthropic_message_delta_usage() {
    let data = json!({
        "delta": {"stop_reason": "end_turn"},
        "usage": {"output_tokens": 80}
    });
    let mut state = AnthropicStreamState::default();
    let event = parse_anthropic_event("message_delta", &data, &mut state);
    assert!(event.is_none());
    assert_eq!(state.output_tokens, 80);
    assert_eq!(state.stop_reason.as_deref(), Some("end_turn"));
}

#[test]
fn test_anthropic_message_stop_produces_done() {
    let mut state = AnthropicStreamState {
        input_tokens: 100,
        output_tokens: 50,
        cache_read_input_tokens: 20,
        stop_reason: Some("end_turn".into()),
    };
    let data = json!({});
    let event = parse_anthropic_event("message_stop", &data, &mut state).unwrap();
    match event {
        StreamEvent::Done(resp) => {
            assert_eq!(resp.usage.prompt_tokens, 100);
            assert_eq!(resp.usage.completion_tokens, 50);
            assert_eq!(resp.usage.cache_hit_tokens, 20);
            assert_eq!(resp.finish_reason.as_deref(), Some("end_turn"));
        }
        _ => panic!("expected Done"),
    }
}

#[test]
fn test_anthropic_unknown_event_returns_none() {
    let mut state = AnthropicStreamState::default();
    let event = parse_anthropic_event("unknown_event", &json!({}), &mut state);
    assert!(event.is_none());
}

#[test]
fn test_anthropic_tool_use_start() {
    let data = json!({
        "index": 0,
        "content_block": {
            "type": "tool_use",
            "id": "toolu_456",
            "name": "write_file"
        }
    });
    let mut state = AnthropicStreamState::default();
    let event = parse_anthropic_event("content_block_start", &data, &mut state).unwrap();
    match event {
        StreamEvent::ToolCallDelta { index, delta } => {
            assert_eq!(index, 0);
            assert_eq!(delta.id.unwrap(), "toolu_456");
            assert_eq!(delta.name.unwrap(), "write_file");
            assert!(delta.arguments_delta.is_empty());
        }
        _ => panic!("expected ToolCallDelta"),
    }
}

// ===== Provider stream_format (3 tests) =====

#[test]
fn test_openai_providers_use_openai_format() {
    use qbird_code_infra::providers::{DeepseekProvider, OllamaProvider, OpenAIProvider, Provider};

    let ds = DeepseekProvider::new(Default::default()).unwrap();
    assert_eq!(ds.stream_format(), StreamFormat::OpenAICompatible);

    let oai = OpenAIProvider::new(Default::default()).unwrap();
    assert_eq!(oai.stream_format(), StreamFormat::OpenAICompatible);

    let ollama = OllamaProvider::new(Default::default()).unwrap();
    assert_eq!(ollama.stream_format(), StreamFormat::OpenAICompatible);
}

#[test]
fn test_anthropic_providers_use_anthropic_format() {
    use qbird_code_infra::providers::{AnthropicProvider, DeepseekAnthropicProvider, Provider};

    let anthropic = AnthropicProvider::new(Default::default()).unwrap();
    assert_eq!(anthropic.stream_format(), StreamFormat::Anthropic);

    let ds_anthropic = DeepseekAnthropicProvider::new(Default::default()).unwrap();
    assert_eq!(ds_anthropic.stream_format(), StreamFormat::Anthropic);
}

// ===== SSE line parsing (2 tests) =====

#[test]
fn test_parse_sse_line_data() {
    match parse_sse_line("data: {\"choices\":[]}") {
        SseLine::Data(s) => assert_eq!(s, "{\"choices\":[]}"),
        _ => panic!("expected Data"),
    }
}

#[test]
fn test_parse_sse_line_event_and_skip() {
    match parse_sse_line("event: message_start") {
        SseLine::Event(s) => assert_eq!(s, "message_start"),
        _ => panic!("expected Event"),
    }
    match parse_sse_line("") {
        SseLine::Skip => {}
        _ => panic!("expected Skip for empty line"),
    }
    match parse_sse_line(": comment") {
        SseLine::Skip => {}
        _ => panic!("expected Skip for comment"),
    }
}

#[test]
fn test_openai_done_signal_returns_none() {
    let chunk = json!("[DONE]");
    let event = parse_openai_chunk(&chunk);
    assert!(event.is_none(), "[DONE] should produce no event");
}

// ===== Integration: mock TCP server → SSE → run_openai_stream =====

#[tokio::test(flavor = "current_thread")]
async fn test_run_openai_stream_with_mock_server() {
    use std::io::Write;
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("local_addr");

    // SSE response: 2 text chunks + usage chunk + [DONE]
    let sse_body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"},\"index\":0}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\" world\"},\"index\":0}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\",\"index\":0}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n",
        "data: [DONE]\n\n",
    );

    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buf = [0u8; 4096];
        let _ = std::io::Read::read(&mut stream, &mut buf);
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            sse_body.len(),
            sse_body
        );
        stream.write_all(resp.as_bytes()).expect("write");
    });

    // Make a streaming HTTP request
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{addr}"))
        .send()
        .await
        .expect("request");

    let (tx, mut rx) = tokio::sync::mpsc::channel(256);
    run_openai_stream(resp, tx).await;

    // Collect all events
    let mut events = Vec::new();
    while let Some(event) = rx.recv().await {
        events.push(event);
    }

    // Should have: TextDelta("Hello"), TextDelta(" world"), Done
    assert!(
        events.len() >= 3,
        "expected ≥3 events, got {}",
        events.len()
    );

    match &events[0] {
        StreamEvent::TextDelta(s) => assert_eq!(s, "Hello"),
        _ => panic!("expected TextDelta('Hello'), got {:?}", &events[0]),
    }
    match &events[1] {
        StreamEvent::TextDelta(s) => assert_eq!(s, " world"),
        _ => panic!("expected TextDelta(' world'), got {:?}", &events[1]),
    }
    // Last event should be Done with usage
    let last = events.last().unwrap();
    match last {
        StreamEvent::Done(resp) => {
            assert_eq!(resp.finish_reason.as_deref(), Some("stop"));
            assert_eq!(resp.usage.prompt_tokens, 10);
            assert_eq!(resp.usage.completion_tokens, 5);
        }
        _ => panic!("expected Done, got {:?}", last),
    }
}
