rust_i18n::i18n!("../../locales", fallback = "en-US");

use rust_i18n::t;

use std::io::{self, BufRead, Write};
use std::sync::Arc;

use clap::Parser;
use qbird_code_agents::skill::{SddProposal, SkillContext, SkillRegistry};
use qbird_code_agents::{ReactLoop, ReactLoopConfig};
use qbird_code_infra::config::{find_config, load_config};
use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::memory::SessionStore;
use qbird_code_infra::providers::{
    AnthropicProvider, DeepseekAnthropicProvider, DeepseekProvider, OllamaProvider, OpenAIProvider,
};
use qbird_code_infra::runtime_overrides::{CliOverrides, RuntimeOverrides};
use qbird_code_models::Message;
use qbird_code_tools::{
    ExecuteCommandTool, GlobTool, ListDirTool, ReadFileTool, SearchCodeTool, ToolRegistry,
    WebFetchTool, WriteFileTool,
};
/// qingbird — Efficient Flow Agent Collaboration Framework
#[derive(Parser)]
#[command(name = "qingbird")]
#[command(version = "0.2.18")]
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

    /// 界面 locale（覆盖 yaml `core.language`）
    #[arg(
        long,
        value_parser = clap::builder::PossibleValuesParser::new(["zh-CN", "en-US"])
    )]
    lang: Option<String>,
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

fn init_llm(
    timeout_secs: u64,
    max_retries: u8,
    retry_backoff_ms: u64,
    provider_result: qbird_code_models::Result<
        impl qbird_code_infra::providers::Provider + 'static,
    >,
) -> (
    HttpLlmClient,
    Box<dyn qbird_code_infra::providers::Provider>,
) {
    let http = match HttpLlmClient::new(timeout_secs, max_retries, retry_backoff_ms) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", t!("err_http_client_init", msg = e));
            std::process::exit(1);
        }
    };
    let provider = match provider_result {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", t!("err_llm_provider_init", msg = e));
            std::process::exit(1);
        }
    };
    (http, Box::new(provider))
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

    let cfg = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e.user_message());
            std::process::exit(1);
        }
    };

    // === 2.5. 初始化 locale（CLI --lang > yaml core.language > 默认） ===
    let resolved_locale_input = cli.lang.as_deref().unwrap_or(cfg.core.language.as_str());
    let active_locale = qbird_code_infra::locale::init(resolved_locale_input);
    tracing::info!(locale = active_locale, "locale activated");

    // --provider/--model/--temperature 命令行参数 → RuntimeOverrides（不 mutate cfg）
    let mut overrides = RuntimeOverrides::from_cli(
        &CliOverrides {
            provider: cli.provider.clone(),
            model: cli.model.clone(),
            temperature: cli.temperature,
        },
        &cfg,
    );
    let model = overrides.resolved_model(&cfg);
    // === 3. 初始化基础设施（根据 overrides.provider 路由） ===
    let active = overrides.current_provider().to_string();
    let (http_client, provider) = match active.as_str() {
        "deepseek" => init_llm(
            cfg.llm.deepseek.timeout_secs,
            cfg.llm.deepseek.max_retries,
            cfg.llm.deepseek.retry_backoff_ms,
            DeepseekProvider::new(cfg.llm.deepseek.clone()),
        ),
        "deepseek-anthropic" => init_llm(
            cfg.llm.deepseek.timeout_secs,
            cfg.llm.deepseek.max_retries,
            cfg.llm.deepseek.retry_backoff_ms,
            DeepseekAnthropicProvider::new(cfg.llm.deepseek.clone()),
        ),
        "ollama" => init_llm(
            cfg.llm.ollama.timeout_secs,
            cfg.llm.ollama.max_retries,
            cfg.llm.ollama.retry_backoff_ms,
            OllamaProvider::new(cfg.llm.ollama.clone()),
        ),
        "openai" => init_llm(
            cfg.llm.openai.timeout_secs,
            cfg.llm.openai.max_retries,
            cfg.llm.openai.retry_backoff_ms,
            OpenAIProvider::new(cfg.llm.openai.clone()),
        ),
        "anthropic" => init_llm(
            cfg.llm.anthropic.timeout_secs,
            cfg.llm.anthropic.max_retries,
            cfg.llm.anthropic.retry_backoff_ms,
            AnthropicProvider::new(cfg.llm.anthropic.clone()),
        ),
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
    // 检查 API Key 是否已配置（仅远程 Provider）
    let env_var = match active.as_str() {
        "deepseek" | "deepseek-anthropic" => {
            let key_empty = cfg
                .llm
                .deepseek
                .api_key
                .as_deref()
                .is_none_or(|s| s.is_empty());
            if key_empty && std::env::var("DEEPSEEK_API_KEY").is_err() {
                Some("DEEPSEEK_API_KEY")
            } else {
                None
            }
        }
        "openai" => {
            let key_empty = cfg
                .llm
                .openai
                .api_key
                .as_deref()
                .is_none_or(|s| s.is_empty());
            if key_empty && std::env::var("OPENAI_API_KEY").is_err() {
                Some("OPENAI_API_KEY")
            } else {
                None
            }
        }
        "anthropic" => {
            let key_empty = cfg
                .llm
                .anthropic
                .api_key
                .as_deref()
                .is_none_or(|s| s.is_empty());
            if key_empty && std::env::var("ANTHROPIC_API_KEY").is_err() {
                Some("ANTHROPIC_API_KEY")
            } else {
                None
            }
        }
        _ => None,
    };
    if let Some(var) = env_var {
        eprintln!("{}", t!("err_api_key_missing", env_var = var));
        std::process::exit(1);
    }

    tracing::info!("Startup: provider={}, model={}", active, model);

    // === 4. 初始化工具注册表 ===
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(ReadFileTool));
    registry.register(Arc::new(WriteFileTool));
    registry.register(Arc::new(ExecuteCommandTool));
    registry.register(Arc::new(SearchCodeTool));
    registry.register(Arc::new(GlobTool));
    registry.register(Arc::new(ListDirTool));
    registry.register(Arc::new(WebFetchTool));
    registry.set_allowed_paths(cfg.security.allowed_paths.clone());
    // 19-07: 风险阈值从 yaml 读，替代历史硬编码 L3
    registry.set_risk_threshold(cfg.security.risk_threshold);
    // 提取 thinking 配置（仅 DeepSeek 支持）
    let (thinking_enabled, thinking_effort) = match active.as_str() {
        "deepseek" | "deepseek-anthropic" => (
            cfg.llm.deepseek.thinking_enabled,
            cfg.llm.deepseek.thinking_effort.clone(),
        ),
        _ => (false, "high".into()),
    };
    let tool_registry = Arc::new(registry);

    // === 4b. 初始化技能注册表 ===
    let mut skill_registry = SkillRegistry::new();
    qbird_code_agents::skill::sdd::register_all(&mut skill_registry);
    let skill_registry = Arc::new(skill_registry);

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
            build_system_message(&tool_registry, &active),
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
                None,
            )
            .await
        {
            Ok(result) => {
                println!("{}", result.content);
            }
            Err(e) => {
                eprintln!("{}", e.user_message());
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
        let mut messages = vec![build_system_message(&tool_registry, &active)];
        let mut cumulative_prompt: u64 = 0;
        let mut cumulative_completion: u64 = 0;
        let mut cumulative_cache_hit: u64 = 0;

        // 初始化 SessionStore
        let session_dirs = dirs::data_dir()
            .map(|p| p.join("qingbird"))
            .unwrap_or_else(|| std::path::PathBuf::from(".qingbird"));
        std::fs::create_dir_all(&session_dirs).ok();
        let db_path = session_dirs.join("sessions.db");
        let session_store = SessionStore::open(&db_path).ok();
        let mut current_session_id = uuid::Uuid::new_v4().to_string();

        let mut context_manager = qbird_code_infra::memory::ContextManager::new(
            "interactive".into(),
            react_loop.config.context_token_limit,
        );

        println!("{}", t!("interactive_banner"));
        println!();

        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();

        // SDD proposal state machine:
        //   /sdd run <input>   → store proposal here, hard_gate_blocked = true
        //   /sdd confirm       → if Some, hard_gate_blocked = false, clear
        //   /sdd status        → show pending + blocked status
        let mut pending_proposal: Option<SddProposal> = None;

        loop {
            print!("> ");
            let _ = io::stdout().flush();

            let line = match lines.next() {
                Some(Ok(line)) => line,
                Some(Err(e)) => {
                    eprintln!("{}", t!("interactive_read_error", msg = e));
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
                        println!(
                            "{}",
                            t!(
                                "interactive_help_provider",
                                name = overrides.current_provider()
                            )
                        );
                        println!("{}", t!("interactive_help_usage"));
                        println!("{}", t!("interactive_help_sessions"));
                        println!("{}", t!("interactive_help_session_load"));
                        println!();
                        println!("{}", t!("interactive_help_sdd_title"));
                        println!("{}", t!("interactive_help_sdd_run"));
                        println!("{}", t!("interactive_help_sdd_confirm"));
                        println!("{}", t!("interactive_help_sdd_status"));
                        println!();
                        println!("{}", t!("interactive_help_undo_planned"));
                        println!("{}", t!("interactive_help_profile_planned"));
                        println!("{}", t!("interactive_help_session_delete_planned"));
                        println!();
                    }
                    "/usage" => {
                        println!("{}", t!("interactive_usage_title"));
                        println!(
                            "{}",
                            t!("interactive_usage_prompt", count = cumulative_prompt)
                        );
                        println!(
                            "{}",
                            t!(
                                "interactive_usage_completion",
                                count = cumulative_completion
                            )
                        );
                        println!(
                            "{}",
                            t!(
                                "interactive_usage_total",
                                count = cumulative_prompt + cumulative_completion
                            )
                        );
                        // 19-07: L1 cache hit 显示（仅当 yaml cache.l1_enabled = true）
                        if cfg.llm.cache.l1_enabled {
                            println!(
                                "{}",
                                t!("interactive_usage_cache_hit", count = cumulative_cache_hit)
                            );
                        }
                    }
                    "/sessions" => {
                        if let Some(ref store) = session_store {
                            match store.list_sessions() {
                                Ok(list) => {
                                    println!("{}", t!("interactive_session_list_title"));
                                    for (id, name, _created, updated, count) in &list {
                                        let time =
                                            chrono::DateTime::from_timestamp_millis(*updated)
                                                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                                                .unwrap_or_default();
                                        println!(
                                            "{}",
                                            t!(
                                                "interactive_session_entry",
                                                id = id,
                                                name = name,
                                                messages = count,
                                                time = time
                                            )
                                        );
                                    }
                                }
                                Err(e) => {
                                    eprintln!("{}", t!("interactive_error_prefix", msg = e));
                                }
                            }
                        } else {
                            eprintln!("{}", t!("err_session_store_unavailable"));
                        }
                    }
                    "/session" => {
                        let sub_parts: Vec<&str> = arg.splitn(2, ' ').collect();
                        let sub_cmd = sub_parts.first().copied().unwrap_or("");
                        let sub_arg = sub_parts.get(1).copied().unwrap_or("");
                        match sub_cmd {
                            "load" => {
                                if sub_arg.is_empty() {
                                    println!("{}", t!("interactive_session_usage"));
                                } else if let Some(ref store) = session_store {
                                    match store.load_messages(sub_arg) {
                                        Ok(loaded) => {
                                            messages = loaded;
                                            println!(
                                                "{}",
                                                t!(
                                                    "interactive_session_loaded",
                                                    id = sub_arg,
                                                    count = messages.len()
                                                )
                                            );
                                            current_session_id = sub_arg.to_string();
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                "Failed to load session {}: {}",
                                                sub_arg,
                                                e
                                            );
                                            eprintln!(
                                                "{}",
                                                t!("interactive_session_not_found", id = sub_arg)
                                            );
                                        }
                                    }
                                }
                            }
                            _ => {
                                println!("{}", t!("interactive_session_usage"));
                            }
                        }
                    }
                    "/model" => {
                        if arg.is_empty() {
                            println!(
                                "{}",
                                t!("interactive_model_current", name = react_loop.config.model)
                            );
                        } else {
                            overrides.set_model(arg.to_string());
                            let resolved = overrides.resolved_model(&cfg);
                            react_loop.config.model = resolved.clone();
                            println!("{}", t!("interactive_model_switched", name = resolved));
                        }
                    }
                    "/sdd" => {
                        if arg.is_empty() {
                            println!("{}", t!("interactive_help_sdd_title"));
                            println!("{}", t!("interactive_help_sdd_run"));
                            println!("{}", t!("interactive_help_sdd_confirm"));
                            println!("{}", t!("interactive_help_sdd_status"));
                        } else {
                            let sub_parts: Vec<&str> = arg.splitn(2, ' ').collect();
                            let sub_cmd = sub_parts[0];
                            let sub_arg = sub_parts.get(1).copied().unwrap_or("");
                            match sub_cmd {
                                "run" => {
                                    let ctx = SkillContext {
                                        session_id: "interactive".into(),
                                        project_path: None,
                                        budget_remaining: None,
                                    };
                                    match skill_registry
                                        .execute(
                                            "sdd-proposal",
                                            serde_json::json!({"userInput": sub_arg}),
                                            ctx,
                                        )
                                        .await
                                    {
                                        Ok(result) => {
                                            // Store proposal in REPL state for /sdd confirm.
                                            if let Ok(proposal) =
                                                serde_json::from_value::<SddProposal>(
                                                    result.output["proposal"].clone(),
                                                )
                                            {
                                                pending_proposal = Some(proposal);
                                            }
                                            println!("{}", result.output);
                                        }
                                        Err(e) => {
                                            eprintln!("{}", t!("interactive_sdd_error", msg = e));
                                        }
                                    }
                                }
                                "confirm" => {
                                    if let Some(mut p) = pending_proposal.take() {
                                        p.hard_gate_blocked = false;
                                        p.status = "confirmed".into();
                                        p.updated_at = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .map(|d| d.as_millis() as i64)
                                            .unwrap_or_default();
                                        println!(
                                            "{}",
                                            t!(
                                                "interactive_sdd_confirmed",
                                                id = p.id,
                                                goal = p.goal
                                            )
                                        );
                                    } else {
                                        eprintln!("{}", t!("interactive_sdd_no_pending"));
                                    }
                                }
                                "status" => {
                                    let skills = skill_registry.list();
                                    println!("{}", t!("sdd_skills_header", count = skills.len()));
                                    for s in &skills {
                                        println!(
                                            "{}",
                                            t!(
                                                "sdd_skill_entry",
                                                level = s.level,
                                                id = s.id,
                                                name = s.name
                                            )
                                        );
                                    }
                                    match &pending_proposal {
                                        Some(p) => {
                                            println!(
                                                "{}",
                                                t!(
                                                    "interactive_sdd_status_pending",
                                                    id = p.id,
                                                    status = p.status,
                                                    hard_gate_blocked = p.hard_gate_blocked
                                                )
                                            );
                                        }
                                        None => {
                                            println!("{}", t!("interactive_sdd_status_idle"));
                                        }
                                    }
                                }
                                _ => {
                                    println!("{}", t!("interactive_unknown_cmd", cmd = line));
                                }
                            }
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
                                    overrides.set_temperature(t);
                                    react_loop.config.temperature = Some(t);
                                    println!("{}", t!("interactive_temp_set", value = t));
                                }
                                _ => println!("{}", t!("interactive_temp_invalid")),
                            }
                        }
                    }
                    "/provider" => {
                        const SUPPORTED: &[&str] = &[
                            "deepseek",
                            "deepseek-anthropic",
                            "ollama",
                            "openai",
                            "anthropic",
                        ];
                        if arg.is_empty() {
                            println!(
                                "{}",
                                t!(
                                    "interactive_provider_current",
                                    name = overrides.current_provider()
                                )
                            );
                        } else if !SUPPORTED.contains(&arg) {
                            println!("{}", t!("interactive_provider_invalid", name = arg));
                        } else {
                            let new = arg.to_string();
                            let needs_model_reset =
                                overrides.model.is_some() && overrides.current_provider() != new;
                            overrides.set_provider(new.clone());
                            let resolved = overrides.resolved_model(&cfg);
                            react_loop.config.model = resolved;
                            println!("{}", t!("interactive_provider_switched", name = new));
                            if needs_model_reset {
                                println!("{}", t!("interactive_provider_reset_model"));
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
                    Some(&mut context_manager),
                )
                .await
            {
                Ok(result) => {
                    cumulative_prompt += result.usage.prompt_tokens;
                    cumulative_completion += result.usage.completion_tokens;
                    cumulative_cache_hit += result.usage.cache_hit_tokens;
                    println!();
                    println!("{}", result.content);
                    println!();
                }
                Err(e) => {
                    eprintln!("{}", t!("interactive_error_prefix", msg = e));
                }
            }

            // 每轮对话后保存 session
            if let Some(ref store) = session_store {
                let _ = store.save_messages(&current_session_id, &messages);
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

        // 退出前保存
        if let Some(ref store) = session_store {
            let _ = store.save_messages(&current_session_id, &messages);
        }

        return;
    }

    // === 7. 无有效子命令 ===
    eprintln!("{}", t!("status_usage"));
    std::process::exit(1);
}
