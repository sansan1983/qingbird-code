use std::io::{self, BufRead, Write};
use std::sync::Arc;

use clap::Parser;
use qbird_code_agents::{ReactLoop, ReactLoopConfig};
use qbird_code_infra::config::{find_config, load_config};
use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::providers::{
    AnthropicProvider, DeepseekProvider, OllamaProvider, OpenAIProvider,
};
use qbird_code_models::Message;
use qbird_code_tools::{
    ExecuteCommandTool, ReadFileTool, SearchCodeTool, ToolRegistry, WriteFileTool,
};
/// qingbird — Efficient Flow Agent Collaboration Framework
#[derive(Parser)]
#[command(name = "qingbird")]
#[command(version = "0.2.0")]
#[command(about = "Efficient Flow Agent Collaboration Framework")]
struct Cli {
    /// 执行单次任务
    #[arg(long, short = 'e')]
    execute: Option<String>,

    /// 交互模式（REPL）
    #[arg(long, short = 'i')]
    interactive: bool,
}

fn build_system_message(registry: &ToolRegistry) -> Message {
    let defs = registry.definitions();
    let tool_names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    Message::system(format!(
        "你是 qingbird，一个高效的编码助手。\n\n可用工具：{}\n\n请通过调用工具来完成任务。每次调用工具后会得到执行结果，根据结果决定下一步。任务完成时给出总结。",
        tool_names.join(", ")
    ))
}

#[tokio::main]
async fn main() {
    // === 1. 初始化日志（走 stderr） ===
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "qingbird=info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    // === 2. 加载配置 ===
    let config_path = find_config().unwrap_or_else(|| {
        eprintln!("No config file found. Place qingbird.yaml in current directory or ~/.qingbird/config.yaml");
        std::process::exit(1);
    });

    let cfg = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    // === 3. 初始化基础设施（根据 cfg.llm.active 路由） ===
    let active = cfg.llm.active.clone();
    let (http_client, provider): (
        HttpLlmClient,
        Box<dyn qbird_code_infra::providers::Provider>,
    ) = match active.as_str() {
        "deepseek" => {
            let h = match HttpLlmClient::new(
                cfg.llm.deepseek.timeout_secs,
                cfg.llm.deepseek.max_retries,
                cfg.llm.deepseek.retry_backoff_ms,
            ) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to initialize HTTP client: {}", e);
                    std::process::exit(1);
                }
            };
            let p = match DeepseekProvider::new(cfg.llm.deepseek.clone()) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to initialize LLM provider: {}", e);
                    std::process::exit(1);
                }
            };
            (
                h,
                Box::new(p) as Box<dyn qbird_code_infra::providers::Provider>,
            )
        }
        "ollama" => {
            let h = match HttpLlmClient::new(cfg.llm.ollama.timeout_secs, 3, 1000) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to initialize HTTP client: {}", e);
                    std::process::exit(1);
                }
            };
            let p = match OllamaProvider::new(cfg.llm.ollama.clone()) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to initialize LLM provider: {}", e);
                    std::process::exit(1);
                }
            };
            (
                h,
                Box::new(p) as Box<dyn qbird_code_infra::providers::Provider>,
            )
        }
        "openai" => {
            let h = match HttpLlmClient::new(cfg.llm.openai.timeout_secs, 3, 1000) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to initialize HTTP client: {}", e);
                    std::process::exit(1);
                }
            };
            let p = match OpenAIProvider::new(cfg.llm.openai.clone()) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to initialize LLM provider: {}", e);
                    std::process::exit(1);
                }
            };
            (
                h,
                Box::new(p) as Box<dyn qbird_code_infra::providers::Provider>,
            )
        }
        "anthropic" => {
            let h = match HttpLlmClient::new(cfg.llm.anthropic.timeout_secs, 3, 1000) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to initialize HTTP client: {}", e);
                    std::process::exit(1);
                }
            };
            let p = match AnthropicProvider::new(cfg.llm.anthropic.clone()) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to initialize LLM provider: {}", e);
                    std::process::exit(1);
                }
            };
            (
                h,
                Box::new(p) as Box<dyn qbird_code_infra::providers::Provider>,
            )
        }
        other => {
            eprintln!("Unknown provider '{other}', expected deepseek/ollama/openai/anthropic");
            std::process::exit(1);
        }
    };
    drop(active);

    // === 4. 初始化工具注册表 ===
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(ReadFileTool));
    registry.register(Arc::new(WriteFileTool));
    registry.register(Arc::new(ExecuteCommandTool));
    registry.register(Arc::new(SearchCodeTool));
    // 提取 thinking 配置（仅 DeepSeek 支持）
    let (thinking_enabled, thinking_effort) = match cfg.llm.active.as_str() {
        "deepseek" => (
            cfg.llm.deepseek.thinking_enabled,
            cfg.llm.deepseek.thinking_effort.clone(),
        ),
        _ => (true, "high".into()),
    };
    let tool_registry = Arc::new(registry);

    // 将 ToolDefinition 转为 OpenAI 兼容的 JSON schema
    let tool_schemas: Vec<serde_json::Value> = tool_registry
        .definitions()
        .into_iter()
        .map(|def| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": def.name,
                    "description": def.description,
                    "parameters": def.parameters,
                }
            })
        })
        .collect();

    // === 5. 单次执行模式 ===
    if let Some(prompt) = cli.execute {
        let react_loop = ReactLoop::new(ReactLoopConfig {
            thinking_enabled,
            thinking_effort: thinking_effort.clone(),
            ..ReactLoopConfig::default()
        });
        let mut messages = vec![build_system_message(&tool_registry), Message::user(&prompt)];

        match react_loop
            .run(
                provider.as_ref(),
                &http_client,
                &mut messages,
                &tool_schemas,
                &tool_registry,
                None,
            )
            .await
        {
            Ok(result) => {
                println!("{}", result.content);
            }
            Err(e) => {
                eprintln!("Task failed: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // === 6. 交互模式（多轮对话） ===
    if cli.interactive {
        let react_loop = ReactLoop::new(ReactLoopConfig {
            thinking_enabled,
            thinking_effort: thinking_effort.clone(),
            ..ReactLoopConfig::default()
        });
        let mut messages = vec![build_system_message(&tool_registry)];
        println!("qingbird interactive mode. Type /quit or /exit to exit.");
        println!();

        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();

        loop {
            print!("> ");
            let _ = io::stdout().flush();

            let line = match lines.next() {
                Some(Ok(line)) => line,
                Some(Err(e)) => {
                    eprintln!("Read error: {}", e);
                    break;
                }
                None => break,
            };

            let line = line.trim().to_string();

            if line == "/quit" || line == "/exit" {
                break;
            }

            if line.is_empty() {
                continue;
            }

            // 追加用户消息到已有对话历史
            messages.push(Message::user(&line));

            match react_loop
                .run(
                    provider.as_ref(),
                    &http_client,
                    &mut messages,
                    &tool_schemas,
                    &tool_registry,
                    None,
                )
                .await
            {
                Ok(result) => {
                    println!();
                    println!("{}", result.content);
                    println!();
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
            // messages 保留全量对话历史，下一轮继续追加
        }

        return;
    }

    // === 7. 无有效子命令 ===
    eprintln!("Usage: qingbird --execute \"prompt\" | qingbird --interactive");
    std::process::exit(1);
}
