rust_i18n::i18n!("../../locales", fallback = "en-US");

use rust_i18n::t;

use std::io::{self, BufRead, Write};
use std::sync::Arc;

use clap::Parser;
use qbird_code_agents::{ReactLoop, ReactLoopConfig};
use qbird_code_infra::config::{find_config, load_config};
use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::providers::{
    AnthropicProvider, DeepseekAnthropicProvider, DeepseekProvider, OllamaProvider, OpenAIProvider,
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

    /// LLM Provider（覆盖 config 中的 llm.active）
    #[arg(
        long,
        value_parser = clap::builder::PossibleValuesParser::new([
            "deepseek", "deepseek-anthropic", "ollama", "openai", "anthropic"
        ])
    )]
    provider: Option<String>,

    /// LLM 模型名称（覆盖 config 中当前 provider 的 default_model）
    #[arg(long)]
    model: Option<String>,

    /// LLM 温度参数（0.0 ~ 2.0，覆盖 config 中的 temperature）
    #[arg(long)]
    temperature: Option<f64>,
}

fn build_system_message(registry: &ToolRegistry, provider_name: &str) -> Message {
    let defs = registry.definitions();
    let tool_names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    Message::system(
        t!(
            "system_prompt",
            provider = provider_name,
            tools = tool_names.join(", ")
        )
        .to_string(),
    )
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
        eprintln!("{}", t!("status_no_config"));
        std::process::exit(1);
    });

    let mut cfg = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", t!("status_config_load_fail", msg = e.to_string()));
            std::process::exit(1);
        }
    };

    // --provider 命令行参数覆盖 config 中的 llm.active
    if let Some(ref provider) = cli.provider {
        cfg.llm.active = provider.clone();
    }

    // 解析当前 provider 的默认模型，--model 可覆盖
    let default_model = match cfg.llm.active.as_str() {
        "deepseek" | "deepseek-anthropic" => &cfg.llm.deepseek.default_model,
        "ollama" => &cfg.llm.ollama.default_model,
        "openai" => &cfg.llm.openai.default_model,
        "anthropic" => &cfg.llm.anthropic.default_model,
        _ => "",
    };
    let model = cli
        .model
        .clone()
        .unwrap_or_else(|| default_model.to_string());
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
        "deepseek-anthropic" => {
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
            let p = match DeepseekAnthropicProvider::new(cfg.llm.deepseek.clone()) {
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
            let h = match HttpLlmClient::new(
                cfg.llm.ollama.timeout_secs,
                cfg.llm.ollama.max_retries,
                cfg.llm.ollama.retry_backoff_ms,
            ) {
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
            let h = match HttpLlmClient::new(
                cfg.llm.openai.timeout_secs,
                cfg.llm.openai.max_retries,
                cfg.llm.openai.retry_backoff_ms,
            ) {
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
            let h = match HttpLlmClient::new(
                cfg.llm.anthropic.timeout_secs,
                cfg.llm.anthropic.max_retries,
                cfg.llm.anthropic.retry_backoff_ms,
            ) {
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
            eprintln!(
                "{}",
                t!(
                    "status_unknown_provider",
                    provider = other,
                    list = "deepseek/deepseek-anthropic/ollama/openai/anthropic"
                )
            );
            std::process::exit(1);
        }
    };
    drop(active);

    tracing::info!(
        "{}",
        t!(
            "status_startup_info",
            provider = cfg.llm.active,
            model = model
        )
    );

    // === 4. 初始化工具注册表 ===
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(ReadFileTool));
    registry.register(Arc::new(WriteFileTool));
    registry.register(Arc::new(ExecuteCommandTool));
    registry.register(Arc::new(SearchCodeTool));
    // 提取 thinking 配置（仅 DeepSeek 支持）
    let (thinking_enabled, thinking_effort) = match cfg.llm.active.as_str() {
        "deepseek" | "deepseek-anthropic" => (
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
            model: model.clone(),
            temperature: cli.temperature,
            thinking_enabled,
            thinking_effort: thinking_effort.clone(),
            ..ReactLoopConfig::default()
        });
        let mut messages = vec![
            build_system_message(&tool_registry, &cfg.llm.active),
            Message::user(&prompt),
        ];

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
                eprintln!("{}", t!("status_task_failed", msg = e.to_string()));
                std::process::exit(1);
            }
        }
        return;
    }

    // === 6. 交互模式（多轮对话） ===
    if cli.interactive {
        const MAX_HISTORY_MSGS: usize = 50;
        let mut react_loop = ReactLoop::new(ReactLoopConfig {
            model: model.clone(),
            temperature: cli.temperature,
            thinking_enabled,
            thinking_effort: thinking_effort.clone(),
            ..ReactLoopConfig::default()
        });
        let mut messages = vec![build_system_message(&tool_registry, &cfg.llm.active)];
        println!("{}", t!("interactive_banner"));
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

            // === 斜杠命令处理 ===
            if line.starts_with('/') {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                let cmd = parts[0];
                let arg = parts.get(1).copied().unwrap_or("");
                match cmd {
                    "/quit" | "/exit" => break,
                    "/help" => {
                        println!();
                        println!("{}", t!("interactive_help_title"));
                        println!("{}", t!("interactive_help_quit"));
                        println!("{}", t!("interactive_help_exit"));
                        println!("{}", t!("interactive_help_help"));
                        println!(
                            "{}",
                            t!("interactive_help_model", name = react_loop.config.model)
                        );
                        println!(
                            "{}",
                            t!(
                                "interactive_help_temp",
                                value = format!("{:?}", react_loop.config.temperature)
                            )
                        );
                        println!();
                    }
                    "/model" => {
                        if arg.is_empty() {
                            println!(
                                "{}",
                                t!("interactive_model_current", name = react_loop.config.model)
                            );
                        } else {
                            react_loop.config.model = arg.to_string();
                            println!("{}", t!("interactive_model_switched", name = arg));
                        }
                    }
                    "/temperature" => {
                        if arg.is_empty() {
                            println!(
                                "{}",
                                t!(
                                    "interactive_temp_current",
                                    value = format!("{:?}", react_loop.config.temperature)
                                )
                            );
                        } else {
                            match arg.parse::<f64>() {
                                Ok(t) if (0.0..=2.0).contains(&t) => {
                                    react_loop.config.temperature = Some(t);
                                    println!("{}", t!("interactive_temp_set", value = t));
                                }
                                _ => println!("{}", t!("interactive_temp_invalid")),
                            }
                        }
                    }
                    _ => {
                        println!("{}", t!("interactive_unknown_cmd", cmd = cmd));
                    }
                }
                continue;
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

            // 上下文窗口管理：超出上限时截断旧消息（保留 system 消息）
            if messages.len() > MAX_HISTORY_MSGS {
                let keep = MAX_HISTORY_MSGS / 2;
                let truncate_start = messages.len() - keep;
                let system = messages[0].clone();
                let remaining = messages[truncate_start..].to_vec();
                messages = std::iter::once(system).chain(remaining).collect();
                eprintln!("{}", t!("interactive_ctx_truncated", count = keep));
            }
        }

        return;
    }

    // === 7. 无有效子命令 ===
    eprintln!("{}", t!("status_usage"));
    std::process::exit(1);
}
