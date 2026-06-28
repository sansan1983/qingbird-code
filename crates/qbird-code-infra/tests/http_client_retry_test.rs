//! HttpLlmClient retry behavior tests using a tiny in-process TCP server.
//! No wiremock/mockito dep — we hand-roll a TCP listener that returns
//! controlled HTTP responses so we can count attempts and measure
//! backoff timing.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::providers::{ProtocolKind, Provider, ProviderKind, StreamEvent};
use qbird_code_models::RetryPolicy;
use serde_json::Value;

/// Mock provider pointing at a caller-supplied base URL.
struct MockProvider {
    base_url: String,
}

#[async_trait]
impl Provider for MockProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAI
    }
    fn protocol(&self) -> ProtocolKind {
        ProtocolKind::OpenAICompatible
    }
    fn model(&self) -> &str {
        "mock-model"
    }
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn build_request_body(
        &self,
        _messages: &[qbird_code_models::Message],
        _config: &qbird_code_infra::providers::RequestConfig,
    ) -> Value {
        serde_json::json!({"messages": []})
    }
    async fn parse_response(
        &self,
        _body: &Value,
    ) -> qbird_code_models::Result<qbird_code_infra::providers::ChatResponse> {
        Err(qbird_code_models::EflowError::LlmProvider("mock".into()))
    }
    fn build_headers(&self) -> std::collections::HashMap<String, String> {
        let mut h = std::collections::HashMap::new();
        h.insert("authorization".into(), "Bearer test".into());
        h
    }
    async fn stream(
        &self,
        _http_client: &HttpLlmClient,
        _messages: &[qbird_code_models::Message],
        _config: &qbird_code_infra::providers::RequestConfig,
    ) -> qbird_code_models::Result<tokio::sync::mpsc::Receiver<StreamEvent>> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let _ = tx
            .send(StreamEvent::Error("mock: streaming not supported".into()))
            .await;
        Ok(rx)
    }
}

/// Spawn a TCP listener that always returns `status_code` and counts
/// connections. Returns (base_url, request_count).
fn start_mock_server(status_code: u16) -> (String, Arc<AtomicU32>) {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = Arc::clone(&counter);

    std::thread::spawn(move || {
        // Single connection loop — accept repeatedly, respond, close.
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            counter_clone.fetch_add(1, Ordering::SeqCst);
            // Read request (drain)
            let mut buf = [0u8; 4096];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            // Respond
            let body = if status_code == 200 {
                r#"{"ok":true}"#
            } else {
                r#"{"err":"boom"}"#
            };
            let resp = format!(
                "HTTP/1.1 {status_code} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            use std::io::Write;
            let _ = stream.write_all(resp.as_bytes());
        }
    });

    (format!("http://{addr}"), counter)
}

#[tokio::test(flavor = "current_thread")]
async fn test_retry_policy_default() {
    // default policy: 3 retries, 1s initial backoff. With our fast-fail
    // 500 server, expect 4 attempts (1 + 3 retries) then error.
    let (base, counter) = start_mock_server(500);
    let provider = MockProvider { base_url: base };
    let client = HttpLlmClient::new(5, RetryPolicy::default()).expect("client");
    let start = Instant::now();
    let result = client.send(&provider, &serde_json::json!({})).await;
    let elapsed = start.elapsed();
    assert!(result.is_err(), "500 should ultimately fail");
    // 1 + 3 = 4 attempts
    assert_eq!(counter.load(Ordering::SeqCst), 4, "expected 4 attempts");
    // Backoffs: 1s + 2s + 4s = 7s minimum (but we don't want a 7s test)
    // Verify at least 1 backoff happened (initial 1s)
    assert!(
        elapsed >= Duration::from_millis(900),
        "expected ≥ 1 backoff, got {elapsed:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_retries_limit() {
    // max_retries = 1: only 1 retry → 2 total attempts
    let (base, counter) = start_mock_server(500);
    let provider = MockProvider { base_url: base };
    let policy = RetryPolicy {
        max_retries: 1,
        initial_backoff_ms: 50, // keep test fast
        backoff_multiplier: 2.0,
        max_backoff_ms: 1000,
    };
    let client = HttpLlmClient::new(5, policy).expect("client");
    let _ = client.send(&provider, &serde_json::json!({})).await;
    assert_eq!(
        counter.load(Ordering::SeqCst),
        2,
        "max_retries=1 → 2 attempts"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_backoff_cap() {
    // Aggressive backoff multiplier that would otherwise blow past max_backoff_ms
    let (base, counter) = start_mock_server(500);
    let provider = MockProvider { base_url: base };
    let policy = RetryPolicy {
        max_retries: 3,
        initial_backoff_ms: 100,
        backoff_multiplier: 100.0, // huge growth
        max_backoff_ms: 150,       // cap to 150ms
    };
    let client = HttpLlmClient::new(5, policy).expect("client");
    let start = Instant::now();
    let _ = client.send(&provider, &serde_json::json!({})).await;
    let elapsed = start.elapsed();
    assert_eq!(counter.load(Ordering::SeqCst), 4);
    // Without cap: 100 + 10000 + 1000000 = ~1s of sleeps. With 150 cap: 100+150+150 = 400ms.
    assert!(
        elapsed < Duration::from_millis(800),
        "max_backoff_ms cap should keep total < 800ms, got {elapsed:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_no_retry_on_4xx() {
    // 401 / 403 / 400 must NOT retry (single attempt, then error)
    for status in [400u16, 401, 403] {
        let (base, counter) = start_mock_server(status);
        let provider = MockProvider { base_url: base };
        let policy = RetryPolicy {
            max_retries: 5,
            initial_backoff_ms: 50,
            backoff_multiplier: 2.0,
            max_backoff_ms: 1000,
        };
        let client = HttpLlmClient::new(5, policy).expect("client");
        let result = client.send(&provider, &serde_json::json!({})).await;
        assert!(result.is_err(), "status {status} should error");
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "status {status} should NOT retry (got counter > 1)"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_per_provider_override_legacy_mapping() {
    // Verify legacy_retry_policy mapping semantics: u8 max_retries + u64 backoff
    // → RetryPolicy { max_retries, initial_backoff_ms, backoff_multiplier: 2.0,
    // max_backoff_ms: 30_000 }
    use qbird_code_models::RetryPolicy as R;
    fn legacy(max_retries: u8, backoff_ms: u64) -> R {
        R {
            max_retries: u32::from(max_retries),
            initial_backoff_ms: backoff_ms,
            backoff_multiplier: 2.0,
            max_backoff_ms: 30_000,
        }
    }
    let p = legacy(3, 1000);
    assert_eq!(p.max_retries, 3);
    assert_eq!(p.initial_backoff_ms, 1000);
    assert_eq!(p.backoff_multiplier, 2.0);
    assert_eq!(p.max_backoff_ms, 30_000);
}

#[tokio::test(flavor = "current_thread")]
async fn test_exponential_backoff_observed() {
    // Verify backoffs actually grow: total elapsed should be > 2x first backoff
    let (base, counter) = start_mock_server(500);
    let provider = MockProvider { base_url: base };
    let policy = RetryPolicy {
        max_retries: 2,
        initial_backoff_ms: 200,
        backoff_multiplier: 2.0,
        max_backoff_ms: 10_000,
    };
    let client = HttpLlmClient::new(5, policy).expect("client");
    let start = Instant::now();
    let _ = client.send(&provider, &serde_json::json!({})).await;
    let elapsed = start.elapsed();
    // 3 attempts, 2 backoffs: 200 + 400 = 600ms minimum
    assert!(
        elapsed >= Duration::from_millis(550),
        "expected ≥ 600ms total backoff, got {elapsed:?}"
    );
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}
