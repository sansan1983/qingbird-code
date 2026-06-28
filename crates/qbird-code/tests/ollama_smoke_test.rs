use qbird_code_agents::react_loop::ReactLoop;
use qbird_code_infra::config::OllamaConfig;
use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::providers::OllamaProvider;
use qbird_code_models::Message;
use qbird_code_tools::ToolRegistry;
use std::sync::Arc;

#[tokio::test]
#[ignore = "需要本地 Ollama 运行"]
async fn ollama_simple_reply_smoke_test() {
    let config = OllamaConfig {
        default_model: "qwen2.5:1.5b".into(),
        ..Default::default()
    };
    let provider = OllamaProvider::new(config).expect("init ollama provider");
    let http = HttpLlmClient::new(
        30,
        qbird_code_models::RetryPolicy {
            max_retries: 1,
            initial_backoff_ms: 1000,
            backoff_multiplier: 2.0,
            max_backoff_ms: 30_000,
        },
    )
    .expect("init http");

    let tool_registry = Arc::new(ToolRegistry::new());
    let tool_schemas = vec![];

    let react = ReactLoop::with_defaults();
    let mut messages = vec![
        Message::system("You are a helpful assistant. Reply in English only."),
        Message::user("Say 'hello, testing'"),
    ];

    let result = react
        .run(
            &provider,
            &http,
            &mut messages,
            &tool_schemas,
            &tool_registry,
            Some(10),
            None,
        )
        .await;

    assert!(
        result.is_ok(),
        "Ollama smoke test failed: {:?}",
        result.err()
    );
    let content = result.unwrap().content.to_lowercase();
    assert!(
        content.contains("hello"),
        "Expected 'hello' in response, got: {}",
        content
    );
}
