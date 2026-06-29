rust_i18n::i18n!("../../locales", fallback = "en-US");

use rust_i18n::t;

use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};

use clap::Parser;
use qbird_code_agents::skill::{SddProposal, SkillContext, SkillRegistry};
use qbird_code_agents::subagent::{SubagentExecutor, SubagentExecutorTrait, load_profiles};
use qbird_code_agents::{DelegateTaskTool, ReactLoop, ReactLoopConfig};
use qbird_code_infra::config::{estimate_cost, find_config, format_cost, load_config};
use qbird_code_infra::config_validate::validate_config;
use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::memory::{MemoryManager, SessionStore};
use qbird_code_infra::profile::Profile;
use qbird_code_infra::providers::{
    AnthropicProvider, DeepseekAnthropicProvider, DeepseekProvider, OllamaProvider, OpenAIProvider,
};
use qbird_code_infra::runtime_overrides::{CliOverrides, RuntimeOverrides};
use qbird_code_models::{Message, RetryPolicy, RiskLevel};
use qbird_code_tools::{
    EditTool, ExecuteCommandTool, GlobTool, ListDirTool, ReadFileTool, SearchCodeTool,
    ToolRegistry, UndoStack, WebFetchTool, WriteFileTool,
};
/// qingbird — Efficient Flow Agent Collaboration Framework
#[derive(Parser)]
#[command(name = "qingbird")]
#[command(version)]
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

    /// Enable streaming mode (typewriter output)
    #[arg(long, default_value = "false")]
    stream: bool,

    /// Disable streaming mode (overrides --stream)
    #[arg(long, default_value = "false")]
    no_stream: bool,

    /// 加载用户 profile 文件 (`<data_dir>/qingbird/profiles/<name>.yaml`)。
    /// 优先级: `--profile` > yaml `profiles.default` > 无 profile。
    #[arg(long)]
    profile: Option<String>,
}

fn build_system_message(registry: &ToolRegistry, provider_name: &str) -> Message {
    Message::system(build_system_prompt_text(registry, provider_name))
}

fn build_system_prompt_text(registry: &ToolRegistry, provider_name: &str) -> String {
    let defs = registry.definitions();
    let tool_names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    t!(
        "system_prompt",
        provider = provider_name,
        tools = tool_names.join(", ")
    )
    .to_string()
}

/// Parse a risk-threshold string ("L0".. "L3") into the enum. None = unknown.
fn parse_risk_threshold(s: &str) -> Option<RiskLevel> {
    match s.to_ascii_uppercase().as_str() {
        "L0" => Some(RiskLevel::L0),
        "L1" => Some(RiskLevel::L1),
        "L2" => Some(RiskLevel::L2),
        "L3" => Some(RiskLevel::L3),
        _ => None,
    }
}

/// Apply a profile onto an existing `Arc<ToolRegistry>` mid-session.
///
/// Clones the inner `ToolRegistry` (cheap — all fields Clone, tools are
/// `Arc<dyn Tool>`), mutates the clone, rewraps it. Replaces the caller's
/// `tool_registry: &mut Arc<ToolRegistry>`. Caller is the sole owner, so
/// `Arc::strong_count == 1` — clone + drop old + arc-new is fine.
///
/// Profile `provider` / `model` fields are merged but do NOT re-init the
/// live `HttpLlmClient` / `Box<dyn Provider>` (out of scope for v0.3.0
/// — see `merge_into` doc). Warnings about this limitation are pushed
/// into `warnings` so the caller can surface them to the user.
fn apply_profile_to_registry(
    profile: &Profile,
    tool_registry: &mut Arc<ToolRegistry>,
    system_prompt: &mut String,
    provider_active: &str,
    model_active: &str,
    warnings: &mut Vec<String>,
) {
    let mut allowed: Option<Vec<String>> = tool_registry.allowed_tools();
    let current_risk = match tool_registry.risk_threshold() {
        RiskLevel::L0 => "L0",
        RiskLevel::L1 => "L1",
        RiskLevel::L2 => "L2",
        RiskLevel::L3 => "L3",
    };
    let mut risk: Option<String> = Some(current_risk.to_string());
    let mut provider = provider_active.to_string();
    let mut model = model_active.to_string();
    profile.merge_into(
        system_prompt,
        &mut allowed,
        &mut risk,
        &mut provider,
        &mut model,
        warnings,
    );

    // Mutate the registry by clone + replace (sole-owner fast path).
    let mut reg = (**tool_registry).clone();
    if let Some(allow) = allowed {
        reg.set_allowed_tools(Some(allow));
    }
    if let Some(r_str) = risk
        && let Some(rl) = parse_risk_threshold(&r_str)
    {
        reg.set_risk_threshold(rl);
    }
    *tool_registry = Arc::new(reg);
}

fn init_llm(
    timeout_secs: u64,
    retry_policy: RetryPolicy,
    provider_result: qbird_code_models::Result<
        impl qbird_code_infra::providers::Provider + 'static,
    >,
) -> (
    HttpLlmClient,
    Box<dyn qbird_code_infra::providers::Provider>,
) {
    let http = match HttpLlmClient::new(timeout_secs, retry_policy) {
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

/// Map a provider's legacy (max_retries, retry_backoff_ms) pair onto the
/// plan-shape `RetryPolicy`. Used during 19-09 transition; later (when
/// cfg adds a top-level `retry_policy` block) this helper will read that
/// block directly.
fn legacy_retry_policy(max_retries: u8, retry_backoff_ms: u64) -> RetryPolicy {
    RetryPolicy {
        max_retries: u32::from(max_retries),
        initial_backoff_ms: retry_backoff_ms,
        backoff_multiplier: 2.0,
        max_backoff_ms: 30_000,
    }
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

    // === 2.6. 配置校验（聚合错误输出，exit code 2） ===
    let validation_errors = validate_config(&cfg);
    if !validation_errors.is_empty() {
        for err in &validation_errors {
            eprintln!("[error] {}", err.message);
        }
        eprintln!(
            "{}",
            t!(
                "err_config_validation_failed",
                count = validation_errors.len()
            )
        );
        std::process::exit(2);
    }

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
            legacy_retry_policy(
                cfg.llm.deepseek.max_retries,
                cfg.llm.deepseek.retry_backoff_ms,
            ),
            DeepseekProvider::new(cfg.llm.deepseek.clone()),
        ),
        "deepseek-anthropic" => init_llm(
            cfg.llm.deepseek.timeout_secs,
            legacy_retry_policy(
                cfg.llm.deepseek.max_retries,
                cfg.llm.deepseek.retry_backoff_ms,
            ),
            DeepseekAnthropicProvider::new(cfg.llm.deepseek.clone()),
        ),
        "ollama" => init_llm(
            cfg.llm.ollama.timeout_secs,
            legacy_retry_policy(cfg.llm.ollama.max_retries, cfg.llm.ollama.retry_backoff_ms),
            OllamaProvider::new(cfg.llm.ollama.clone()),
        ),
        "openai" => init_llm(
            cfg.llm.openai.timeout_secs,
            legacy_retry_policy(cfg.llm.openai.max_retries, cfg.llm.openai.retry_backoff_ms),
            OpenAIProvider::new(cfg.llm.openai.clone()),
        ),
        "anthropic" => init_llm(
            cfg.llm.anthropic.timeout_secs,
            legacy_retry_policy(
                cfg.llm.anthropic.max_retries,
                cfg.llm.anthropic.retry_backoff_ms,
            ),
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

    // Streaming mode: --stream enables, --no-stream disables (default: off)
    let stream_enabled = cli.stream && !cli.no_stream;

    // === 4. 初始化工具注册表 ===
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(ReadFileTool));
    registry.register(Arc::new(WriteFileTool));
    registry.register(Arc::new(ExecuteCommandTool));
    registry.register(Arc::new(SearchCodeTool));
    registry.register(Arc::new(GlobTool));
    registry.register(Arc::new(ListDirTool));
    registry.register(Arc::new(WebFetchTool));

    // Undo stack lives outside ToolRegistry so profile switches cannot clear it.
    let undo_stack: Arc<Mutex<UndoStack>> = Arc::new(Mutex::new(UndoStack::new()));
    registry.register(Arc::new(
        EditTool::new().with_undo_stack(undo_stack.clone()),
    ));

    registry.set_allowed_paths(cfg.security.allowed_paths.clone());
    // 19-07: 风险阈值从 yaml 读，替代历史硬编码 L3
    registry.set_risk_threshold(cfg.security.risk_threshold);

    // === 4a. 解析 + 应用 profile（Task 30-02）===
    // 优先级: --profile CLI flag > yaml profiles.default > 无
    // 此时 LLM 已 init（确定性）、registry 刚组装好；profile 仍可在
    // model=... / provider=... 已被 reads 的状态下覆盖。
    let profile_dir = Profile::default_dir();
    // First startup: create sample profiles if the directory is empty.
    if let Err(e) = Profile::create_sample_profiles(&profile_dir) {
        tracing::warn!("Failed to create sample profiles: {}", e);
    }
    let resolved_profile_name: Option<String> =
        match (&cli.profile, cfg.profiles.default.is_empty()) {
            (Some(name), _) => Some(name.clone()),
            (None, false) => Some(cfg.profiles.default.clone()),
            (None, true) => None,
        };
    let mut active_profile: Option<String> = None;
    let mut active_allowed_tools: Option<Vec<String>> = None;
    let mut active_risk_override: Option<String> = None;
    let mut profile_overridden_system_prompt: Option<String> = None;

    if let Some(ref pname) = resolved_profile_name {
        match Profile::load(&profile_dir, pname) {
            Ok(p) => {
                tracing::info!(profile = %p.name, "profile loaded");
                let mut sp = build_system_prompt_text(&registry, &active);
                let mut provider_active = active.clone();
                let mut model_active = model.clone();
                let mut profile_warnings: Vec<String> = Vec::new();
                p.merge_into(
                    &mut sp,
                    &mut active_allowed_tools,
                    &mut active_risk_override,
                    &mut provider_active,
                    &mut model_active,
                    &mut profile_warnings,
                );
                if let Some(ref allow) = active_allowed_tools {
                    registry.set_allowed_tools(Some(allow.clone()));
                }
                if let Some(ref r) = active_risk_override
                    && let Some(rl) = parse_risk_threshold(r)
                {
                    registry.set_risk_threshold(rl);
                }
                profile_overridden_system_prompt = Some(sp);
                active_profile = Some(p.name.clone());
                // Surface provider/model-restart-required warnings to the user.
                // Without this, a user with `provider: ollama` in their profile
                // silently gets `deepseek` (LLM was already constructed above).
                for w in &profile_warnings {
                    eprintln!("{w}");
                    tracing::warn!("{w}");
                }
            }
            Err(e) => {
                eprintln!("{}", e.user_message());
                std::process::exit(1);
            }
        }
    }

    // 提取 thinking 配置（仅 DeepSeek 支持）
    let (thinking_enabled, thinking_effort) = match active.as_str() {
        "deepseek" | "deepseek-anthropic" => (
            cfg.llm.deepseek.thinking_enabled,
            cfg.llm.deepseek.thinking_effort.clone(),
        ),
        _ => (false, "high".into()),
    };

    // === 4c. 构造 SubagentExecutor + 注册 DelegateTaskTool (v0.3.1) ===
    // temp_registry 是 registry 的 snapshot（此时已含所有 profile 应用的
    // allowed_tools / risk_threshold / allowed_paths），subagent executor
    // 共享主 agent 的安全设置。delegate_task_tool 单独注册到主 registry
    // （不进入 subagent 的 registry，避免子代理递归派发）。
    let temp_registry_arc = Arc::new(registry.clone());
    let subagent_profiles = match load_profiles(None) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e.user_message());
            std::process::exit(1);
        }
    };
    let subagent_executor = match SubagentExecutor::builder()
        .profiles(subagent_profiles)
        .base_config(ReactLoopConfig {
            model: model.clone(),
            ..ReactLoopConfig::default()
        })
        .tool_registry(temp_registry_arc)
        .build()
    {
        Ok(e) => Arc::new(e),
        Err(e) => {
            eprintln!("{}", e.user_message());
            std::process::exit(1);
        }
    };
    let delegate_task_tool =
        DelegateTaskTool::new(subagent_executor.clone() as Arc<dyn SubagentExecutorTrait>);
    registry.register(Arc::new(delegate_task_tool));
    let mut tool_registry = Arc::new(registry);

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
            stream_enabled,
            subagent_executor: Some(subagent_executor.clone()),
            ..ReactLoopConfig::default()
        });
        let mut messages = vec![
            match &profile_overridden_system_prompt {
                Some(sp) => Message::system(sp.clone()),
                None => build_system_message(&tool_registry, &active),
            },
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
                None, // no memory manager in --execute mode
            )
            .await
        {
            Ok(result) => {
                println!("{}", result.content);
                // 30-04: cost line
                let (cost_input, cost_output) = match active.as_str() {
                    "deepseek" | "deepseek-anthropic" => (
                        cfg.llm.deepseek.cost_per_million_input_tokens,
                        cfg.llm.deepseek.cost_per_million_output_tokens,
                    ),
                    "ollama" => (
                        cfg.llm.ollama.cost_per_million_input_tokens,
                        cfg.llm.ollama.cost_per_million_output_tokens,
                    ),
                    "openai" => (
                        cfg.llm.openai.cost_per_million_input_tokens,
                        cfg.llm.openai.cost_per_million_output_tokens,
                    ),
                    "anthropic" => (
                        cfg.llm.anthropic.cost_per_million_input_tokens,
                        cfg.llm.anthropic.cost_per_million_output_tokens,
                    ),
                    _ => (0.0, 0.0),
                };
                if let Some(usd) = estimate_cost(
                    result.usage.prompt_tokens,
                    result.usage.completion_tokens,
                    result.usage.cache_hit_tokens,
                    cost_input,
                    cost_output,
                ) {
                    println!("[cost] ${:.4} USD", usd);
                }
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
            stream_enabled,
            subagent_executor: Some(subagent_executor.clone()),
            ..ReactLoopConfig::default()
        });
        let mut messages = vec![match &profile_overridden_system_prompt {
            Some(sp) => Message::system(sp.clone()),
            None => build_system_message(&tool_registry, &active),
        }];
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
        let archive_dir = dirs::data_dir()
            .map(|p| p.join("qingbird").join("sessions.archive"))
            .unwrap_or_else(|| std::path::PathBuf::from(".qingbird/sessions.archive"));
        std::fs::create_dir_all(&archive_dir).ok();
        if let Some(ref store) = session_store {
            match store.should_cleanup(cfg.memory.cleanup_interval_hours) {
                Ok(true) => {
                    if let Err(e) = store.cleanup_old_sessions(50) {
                        tracing::warn!("Session LRU cleanup failed: {}", e);
                    } else {
                        let _ = store.mark_cleanup();
                    }
                }
                Ok(false) => {
                    tracing::info!(
                        "Session cleanup throttled (interval {}h)",
                        cfg.memory.cleanup_interval_hours
                    );
                }
                Err(e) => {
                    tracing::warn!("should_cleanup check failed: {}", e);
                }
            }
        }
        let mut current_session_id = uuid::Uuid::new_v4().to_string();

        let mut context_manager = qbird_code_infra::memory::ContextManager::new(
            "interactive".into(),
            react_loop.config.context_token_limit,
        );

        // 19-02: init MemoryManager (XDG default path, auto-create parent)
        let memory_manager = match MemoryManager::default_db_path() {
            Ok(db_path) => match MemoryManager::open(&db_path) {
                Ok(mm) => {
                    tracing::info!("MemoryManager opened: {}", db_path.display());
                    Some(Arc::new(mm))
                }
                Err(e) => {
                    tracing::warn!("MemoryManager open failed (disabled): {}", e);
                    None
                }
            },
            Err(e) => {
                tracing::warn!("MemoryManager path unavailable (disabled): {}", e);
                None
            }
        };

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
                        println!("{}", t!("interactive_help_session_delete"));
                        println!("{}", t!("interactive_help_session_rename"));
                        println!();
                        println!("{}", t!("interactive_help_sdd_title"));
                        println!("{}", t!("interactive_help_sdd_run"));
                        println!("{}", t!("interactive_help_sdd_confirm"));
                        println!("{}", t!("interactive_help_sdd_status"));
                        println!();
                        println!("{}", t!("interactive_help_undo"));
                        println!("{}", t!("interactive_help_profile"));
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
                        if cumulative_cache_hit > 0 {
                            println!(
                                "{}",
                                t!("interactive_usage_cache_hit", count = cumulative_cache_hit)
                            );
                        }
                        // 30-04: cost display
                        let (cost_input, cost_output) = match active.as_str() {
                            "deepseek" | "deepseek-anthropic" => (
                                cfg.llm.deepseek.cost_per_million_input_tokens,
                                cfg.llm.deepseek.cost_per_million_output_tokens,
                            ),
                            "ollama" => (
                                cfg.llm.ollama.cost_per_million_input_tokens,
                                cfg.llm.ollama.cost_per_million_output_tokens,
                            ),
                            "openai" => (
                                cfg.llm.openai.cost_per_million_input_tokens,
                                cfg.llm.openai.cost_per_million_output_tokens,
                            ),
                            "anthropic" => (
                                cfg.llm.anthropic.cost_per_million_input_tokens,
                                cfg.llm.anthropic.cost_per_million_output_tokens,
                            ),
                            _ => (0.0, 0.0),
                        };
                        let is_zh = active_locale.starts_with("zh");
                        match estimate_cost(
                            cumulative_prompt,
                            cumulative_completion,
                            cumulative_cache_hit,
                            cost_input,
                            cost_output,
                        ) {
                            Some(usd) => {
                                println!("{}", format_cost(usd, is_zh));
                            }
                            None => {
                                println!("{}", t!("interactive_usage_cost_unknown"));
                            }
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
                            "delete" => {
                                if sub_arg.is_empty() {
                                    println!("{}", t!("interactive_session_delete_usage"));
                                } else if let Some(ref store) = session_store {
                                    match store.delete(sub_arg, &archive_dir) {
                                        Ok(()) => {
                                            println!(
                                                "{}",
                                                t!("interactive_session_deleted", id = sub_arg)
                                            );
                                            if current_session_id == sub_arg {
                                                current_session_id =
                                                    uuid::Uuid::new_v4().to_string();
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("{}", e.user_message());
                                        }
                                    }
                                } else {
                                    eprintln!("{}", t!("err_session_store_unavailable"));
                                }
                            }
                            "rename" => {
                                if sub_arg.is_empty() {
                                    println!("{}", t!("interactive_session_rename_usage"));
                                } else {
                                    let rename_parts: Vec<&str> = sub_arg.splitn(2, ' ').collect();
                                    let id = rename_parts.first().copied().unwrap_or("");
                                    let new_name =
                                        rename_parts.get(1).copied().unwrap_or("").trim();
                                    if id.is_empty() || new_name.is_empty() {
                                        println!(
                                            "{}",
                                            t!("interactive_session_rename_missing_name")
                                        );
                                    } else if let Some(ref store) = session_store {
                                        match store.rename(id, new_name) {
                                            Ok(()) => {
                                                println!(
                                                    "{}",
                                                    t!(
                                                        "interactive_session_renamed",
                                                        id = id,
                                                        name = new_name
                                                    )
                                                );
                                            }
                                            Err(e) => {
                                                eprintln!("{}", e.user_message());
                                            }
                                        }
                                    } else {
                                        eprintln!("{}", t!("err_session_store_unavailable"));
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
                    "/profile" => {
                        if arg.is_empty() {
                            match &active_profile {
                                Some(name) => {
                                    println!(
                                        "{}",
                                        t!("interactive_profile_current", name = name.as_str())
                                    );
                                }
                                None => {
                                    println!("{}", t!("interactive_profile_usage"));
                                }
                            }
                        } else if arg == "list" {
                            match Profile::list(&profile_dir) {
                                Ok(list) => {
                                    println!("{}", t!("interactive_profile_list_title"));
                                    for n in &list {
                                        println!("  {n}");
                                    }
                                }
                                Err(e) => {
                                    eprintln!("{}", e.user_message());
                                }
                            }
                        } else {
                            // Switch profile mid-session.
                            match Profile::load(&profile_dir, arg) {
                                Ok(p) => {
                                    // Re-apply onto messages[0] (system prompt)
                                    // and tool_registry.set_allowed_tools +
                                    // set_risk_threshold (via clone+replace).
                                    let mut sp = if let Some(m) = messages.first() {
                                        m.content.clone()
                                    } else {
                                        build_system_prompt_text(&tool_registry, &active)
                                    };
                                    let mut switch_warnings: Vec<String> = Vec::new();
                                    apply_profile_to_registry(
                                        &p,
                                        &mut tool_registry,
                                        &mut sp,
                                        &active,
                                        &model,
                                        &mut switch_warnings,
                                    );
                                    // Re-write messages[0] with the new prompt.
                                    if let Some(m) = messages.first_mut() {
                                        m.content = sp.clone();
                                    }
                                    active_profile = Some(p.name.clone());
                                    println!(
                                        "{}",
                                        t!("interactive_profile_loaded", name = p.name.as_str())
                                    );
                                    // Mid-session provider/model overrides cannot
                                    // re-init the live LLM client. Surface this
                                    // explicitly so the user understands why
                                    // their `provider:` change doesn't take effect.
                                    for w in &switch_warnings {
                                        eprintln!("{w}");
                                        tracing::warn!("{w}");
                                    }
                                    if !switch_warnings.is_empty() {
                                        eprintln!("{}", t!("interactive_profile_restart_note"));
                                    }
                                }
                                Err(e) => {
                                    eprintln!("{}", e.user_message());
                                }
                            }
                        }
                    }
                    "/undo" => match undo_stack.lock() {
                        Ok(mut stack) => match stack.pop() {
                            Some(entry) => {
                                let path = entry.path.clone();
                                let content = entry.previous_content.clone();
                                drop(stack);
                                match std::fs::write(&path, &content) {
                                    Ok(()) => {
                                        println!(
                                            "{}",
                                            t!("interactive_undo_success", path = path.display())
                                        );
                                    }
                                    Err(e) => {
                                        eprintln!("{}", t!("err_io", msg = e.to_string()));
                                    }
                                }
                            }
                            None => {
                                eprintln!("{}", t!("err_undo_stack_empty"));
                            }
                        },
                        Err(e) => {
                            eprintln!("{}", t!("err_undo_lock_failed", msg = e.to_string()));
                        }
                    },
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
                    memory_manager.clone(),
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

            // 19-01: 上下文窗口管理走 ContextManager 的 token budget（替代
            // 历史的 50 条硬截断）。cm 跟踪每轮 add_chat_message，budget
            // 触发时按 token 截断（保留 system + 最近 N 条）。
            if messages.len() > MAX_HISTORY_MSGS {
                let budget = react_loop.config.context_token_limit;
                let within = context_manager.get_messages_within_budget(budget);
                if within.len() < context_manager.get_message_count() {
                    // 重组 messages：保留 system + within 涵盖的最近条目
                    let system = messages.first().cloned();
                    let kept_indices: std::collections::HashSet<String> = within
                        .iter()
                        .map(|c| format!("{}|{}", c.role, c.content))
                        .collect();
                    let recent: Vec<Message> = messages
                        .iter()
                        .rev()
                        .filter(|m| {
                            let key = format!("{}|{}", m.role_str(), m.content);
                            kept_indices.contains(&key)
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                    let recent_len = recent.len();
                    messages = if let Some(sys) = system {
                        std::iter::once(sys).chain(recent).collect()
                    } else {
                        recent
                    };
                    eprintln!("{}", t!("interactive_ctx_truncated", count = recent_len));
                } else {
                    // 19-01: cm 没建议截断，保留历史 50 条硬截断兜底
                    let keep = MAX_HISTORY_MSGS / 2;
                    let truncate_start = messages.len() - keep;
                    let system = messages[0].clone();
                    let remaining = messages[truncate_start..].to_vec();
                    messages = std::iter::once(system).chain(remaining).collect();
                    eprintln!("{}", t!("interactive_ctx_truncated", count = keep));
                }
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
