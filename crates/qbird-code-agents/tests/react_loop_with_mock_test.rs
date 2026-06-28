use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::json;

use qbird_code_infra::config::DeepseekConfig;
use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::providers::DeepseekProvider;
use qbird_code_models::Message;
use qbird_code_tools::{ReadFileTool, ToolRegistry};

/// 简易 mock HTTP server（阻塞线程，全读取请求体）
fn start_mock_server() -> (String, Arc<AtomicUsize>) {
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().unwrap();
    let addr_str = format!("http://{}", addr);

    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let _ = tx.send(());
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => break,
            };
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(1)))
                .ok();
            stream
                .set_write_timeout(Some(std::time::Duration::from_secs(1)))
                .ok();

            // 读取完整 HTTP 请求（含 body），防止 drop 时残余数据导致 RST
            let mut buf = Vec::new();
            let mut content_length: Option<usize> = None;
            loop {
                let mut chunk = [0u8; 4096];
                match stream.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        buf.extend_from_slice(&chunk[..n]);
                        // 检测请求结束
                        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                            // 解析 Content-Length
                            if content_length.is_none() {
                                let header = String::from_utf8_lossy(&buf[..pos]);
                                for line in header.lines() {
                                    if let Some(len_str) = line
                                        .strip_prefix("Content-Length:")
                                        .or_else(|| line.strip_prefix("content-length:"))
                                    {
                                        content_length = len_str.trim().parse::<usize>().ok();
                                    }
                                }
                            }
                            let header_end = pos + 4;
                            let body_len = buf.len().saturating_sub(header_end);
                            if let Some(cl) = content_length {
                                if body_len >= cl {
                                    break;
                                }
                            } else {
                                // 没有 Content-Length 就假定 headers 读完
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }

            let count = c.fetch_add(1, Ordering::SeqCst);
            let response_body = match count {
                0 => json!({
                    "id": "chatcmpl-001",
                    "object": "chat.completion",
                    "choices": [{
                        "index": 0,
                        "finish_reason": "tool_calls",
                        "message": {
                            "role": "assistant",
                            "content": "",
                            "tool_calls": [{
                                "id": "call_read_file",
                                "type": "function",
                                "function": {
                                    "name": "read_file",
                                    "arguments": r#"{"path": "Cargo.toml"}"#
                                }
                            }]
                        }
                    }],
                    "usage": { "prompt_tokens": 50, "completion_tokens": 20 }
                }),
                _ => json!({
                    "id": "chatcmpl-002",
                    "object": "chat.completion",
                    "choices": [{
                        "index": 0,
                        "finish_reason": "stop",
                        "message": {
                            "role": "assistant",
                            "content": "已完成任务，Cargo.toml 是一个 Rust 项目的配置文件。"
                        }
                    }],
                    "usage": { "prompt_tokens": 100, "completion_tokens": 30 }
                }),
            };

            let body_str = response_body.to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body_str.len(),
                body_str
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    rx.recv().expect("mock server should start");

    (addr_str, counter)
}

#[tokio::test]
async fn test_agent_full_react_loop() {
    let (base_url, call_count) = start_mock_server();

    let cfg = DeepseekConfig {
        api_key: Some("test-key".into()),
        base_url,
        timeout_secs: 30,
        max_retries: 0,
        retry_backoff_ms: 1,
        ..Default::default()
    };

    let http_client = HttpLlmClient::new(
        cfg.timeout_secs,
        qbird_code_models::RetryPolicy {
            max_retries: u32::from(cfg.max_retries),
            initial_backoff_ms: cfg.retry_backoff_ms,
            backoff_multiplier: 2.0,
            max_backoff_ms: 30_000,
        },
    )
    .expect("Failed to create HTTP client");
    let provider = DeepseekProvider::new(cfg).expect("Failed to create provider");

    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(ReadFileTool));
    let tool_registry = Arc::new(registry);

    let tool_schemas: Vec<serde_json::Value> = tool_registry
        .definitions()
        .into_iter()
        .map(|def| {
            json!({
                "type": "function",
                "function": {
                    "name": def.name,
                    "description": def.description,
                    "parameters": def.parameters,
                }
            })
        })
        .collect();

    let react_loop = qbird_code_agents::ReactLoop::with_defaults();
    let mut messages = vec![
        Message::system("你是 qingbird 测试助手，请使用工具完成任务。"),
        Message::user("请读取 Cargo.toml 文件并总结"),
    ];

    let result = react_loop
        .run(
            &provider,
            &http_client,
            &mut messages,
            &tool_schemas,
            &tool_registry,
            Some(10),
            None,
            None,
        )
        .await
        .expect("Agent loop should succeed");

    assert!(!result.content.is_empty(), "Agent should produce output");
    assert!(
        result.content.contains("Cargo.toml"),
        "Output should mention Cargo.toml, got: {}",
        result.content
    );
    assert!(
        call_count.load(Ordering::SeqCst) >= 2,
        "Should call LLM at least 2 times (tool call + completion)"
    );

    // 清理 mock server（drop TcpListener 自动关闭）
}
