use std::sync::Arc;

// 在 bin crate 中也调用 i18n!() 以生成 _rust_i18n_t! 宏（让 main.rs 里的 t!() 可用）
rust_i18n::i18n!("locales", fallback = "en-US");

use qingbird_code::application::concierge::Concierge;
use qingbird_code::application::orchestrator::Orchestrator;
use qingbird_code::capability::pool::SubagentPool;
use qingbird_code::capability::tools::{
    ToolRegistry,
    command::ExecuteCommandTool,
    file::{ReadFileTool, WriteFileTool},
    search::SearchCodeTool,
};
use qingbird_code::common::types::ModelTier;
use qingbird_code::infrastructure::config;
use qingbird_code::infrastructure::event::{Event, EventChannel};
use qingbird_code::infrastructure::llm::LlmRouter;
use qingbird_code::infrastructure::locale;
use qingbird_code::infrastructure::memory::CompositeMemory;
use qingbird_code::infrastructure::profile::ProfileRegistry;
use qingbird_code::interaction::InteractionLayer;
use qingbird_code::interaction::cli::{Cli, Command, SessionAction};
use qingbird_code::interaction::tui::TuiBackend;
use rust_i18n::t;

#[tokio::main]
async fn main() {
    // 初始化日志（走 stderr —— 契约冻结 v1.3.0 起：stdout 永远 JSON 契约，
    // stderr 永远人类可读。spec B2 §3.5 ADR-0017）
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "qingbird=info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse_args();

    // 子命令路由
    match cli.command {
        Some(Command::Init) => {
            let exit_code = qingbird_code::cli::init::run();
            std::process::exit(exit_code);
        }
        Some(Command::Tui) => {
            // qingbird tui：检测配置 → 未配置则文本配置 → 已配置则进 TUI
            if !qingbird_code::cli::config::check_llm_configured() {
                let code = qingbird_code::cli::config::run();
                if code != 0 {
                    std::process::exit(code);
                }
                println!("配置已保存。重新运行 qingbird tui 启动 TUI。");
                return;
            }
            run_tui().await;
            return;
        }
        Some(Command::Session {
            action: SessionAction::Start { config, lang },
        }) => {
            let config = config.map(std::path::PathBuf::from);
            let exit_code = qingbird_code::cli::start::run(config, lang).await;
            std::process::exit(exit_code);
        }
        None => {
            // eflow（无子命令）
            if !qingbird_code::cli::config::check_llm_configured() {
                let code = qingbird_code::cli::config::run();
                if code != 0 {
                    std::process::exit(code);
                }
                println!("配置已保存。重新运行 qingbird 开始使用。");
                return;
            }
            // CLI 对话模式（--execute / --show-config / --list-profiles 走现有逻辑）
            if cli.execute.is_some() || cli.show_config || cli.list_profiles || cli.lang.is_some() {
                // 延后处理——走下面的配置加载 + 执行路径
            } else {
                println!("LLM 已配置。运行 qingbird --execute \"...\" 执行任务");
                println!("或 qingbird tui 启动 TUI 对话模式。");
                return;
            }
        }
    }

    // --- 以下代码在 --execute / --show-config / --list-profiles 时执行 ---

    // 加载配置
    let config_path = config::find_config().unwrap_or_else(|| {
        eprintln!("{}", t!("cli_no_config"));
        std::path::PathBuf::from("qingbird.yaml")
    });

    let cfg = match config::load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}: {}", t!("err_config_load", msg = e.to_string()), e);
            return;
        }
    };

    // 启动时优先 --lang，回退 config.core.language
    let lang = cli.lang.as_deref().or(Some(cfg.core.language.as_str()));
    locale::init(lang);

    // 初始化基础设施
    let events = EventChannel::new();

    // v1.3 起：传 provider_dir 给 from_config
    let provider_dir = dirs::config_dir()
        .map(|p| p.join("qingbird").join("providers"))
        .unwrap_or_else(|| std::path::PathBuf::from("./providers"));
    let _ = std::fs::create_dir_all(&provider_dir);
    let llm = match LlmRouter::from_config(&cfg, &provider_dir) {
        Ok(l) => Arc::new(tokio::sync::Mutex::new(l)),
        Err(e) => {
            eprintln!("{}: {}", t!("err_llm_init", msg = e.to_string()), e);
            return;
        }
    };

    // v1.1 Task B7: 启动时打印 L2 状态
    if cfg.llm.cache.l2_enabled {
        tracing::info!(
            "{}",
            t!("status_l2_enabled", days = cfg.llm.cache.l2_ttl_days)
        );
    } else {
        tracing::info!("{}", t!("status_l2_disabled"));
    }

    // 初始化工具注册表
    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(Arc::new(ReadFileTool));
    tool_registry.register(Arc::new(WriteFileTool));
    tool_registry.register(Arc::new(ExecuteCommandTool));
    tool_registry.register(Arc::new(SearchCodeTool));
    let tools = Arc::new(tool_registry);

    // 初始化记忆
    let memory = match CompositeMemory::new(
        cfg.memory.working_memory_limit,
        std::path::Path::new(&cfg.memory.project_db_path),
        std::path::Path::new(&cfg.memory.user_db_path),
    ) {
        Ok(m) => Arc::new(tokio::sync::Mutex::new(m)),
        Err(e) => {
            eprintln!("{}: {}", t!("err_memory_init", msg = e.to_string()), e);
            return;
        }
    };

    // 初始化 Profile
    let mut profiles = ProfileRegistry::new();
    if let Err(e) = profiles.load_profiles(std::path::Path::new("profiles")) {
        tracing::warn!("Failed to load profiles: {}", e);
    }
    let profiles = Arc::new(tokio::sync::RwLock::new(profiles));

    // 初始化 Orchestrator
    let pool = Arc::new(SubagentPool::start(4));
    let orchestrator =
        Orchestrator::with_pool(llm.clone(), tools.clone(), events.clone(), pool.clone());
    let orchestrator = Arc::new(tokio::sync::Mutex::new(orchestrator));

    // 初始化 Concierge
    let mut concierge = Concierge::new(
        events.clone(),
        memory.clone(),
        orchestrator.clone(),
        llm.clone(),
        cfg.profiles.default.clone(),
    );

    // 注册 6 个斜杠命令
    if let Err(e) = register_slash_commands(&mut concierge) {
        eprintln!("{}: {}", t!("err_slash_register", msg = e.to_string()), e);
        std::process::exit(1);
    }

    let concierge = std::sync::Arc::new(tokio::sync::Mutex::new(concierge));

    // 显示配置
    if cli.show_config {
        println!("Active profile: {}", cfg.profiles.default);
        println!(
            "LLM Strong: {:?}",
            llm.lock().await.provider_for(ModelTier::Strong)
        );
        return;
    }

    // 列出 Profile
    if cli.list_profiles {
        let p = profiles.read().await;
        println!("Available profiles: {:?}", p.list_profiles());
        return;
    }

    // 单次执行模式
    if let Some(task) = cli.execute {
        let mut rx = events.subscribe();
        let ack = concierge.lock().await.handle_input(task).await;
        println!("{ack}");

        let result = tokio::time::timeout(std::time::Duration::from_secs(60), async {
            loop {
                match rx.recv().await {
                    Ok(Event::TaskCompleted { summary, .. }) => return Ok(summary),
                    Ok(Event::TaskFailed { error, .. }) => return Err(error),
                    Ok(_) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        return Err("event channel closed".to_string());
                    }
                }
            }
        })
        .await;

        match result {
            Ok(Ok(summary)) => println!("{summary}"),
            Ok(Err(e)) => eprintln!("Task failed: {e}"),
            Err(_) => eprintln!("Timeout waiting for task completion (60s)"),
        }
        pool.shutdown().await;
        return;
    }

    // 不应到达这里——无子命令时上面已处理
    pool.shutdown().await;
}

/// TUI 模式启动
async fn run_tui() {
    // 基础设施已由调用方初始化
    let provider_dir = dirs::config_dir()
        .map(|p| p.join("qingbird").join("providers"))
        .unwrap_or_else(|| std::path::PathBuf::from("./providers"));
    let config_path = config::find_config().unwrap_or_else(|| {
        eprintln!("{}", t!("cli_no_config"));
        std::path::PathBuf::from("qingbird.yaml")
    });
    let cfg = match config::load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}: {}", t!("err_config_load", msg = e.to_string()), e);
            return;
        }
    };
    let events = EventChannel::new();
    let _ = std::fs::create_dir_all(&provider_dir);
    let llm = match LlmRouter::from_config(&cfg, &provider_dir) {
        Ok(l) => Arc::new(tokio::sync::Mutex::new(l)),
        Err(e) => {
            eprintln!("{}: {}", t!("err_llm_init", msg = e.to_string()), e);
            return;
        }
    };

    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(Arc::new(ReadFileTool));
    tool_registry.register(Arc::new(WriteFileTool));
    tool_registry.register(Arc::new(ExecuteCommandTool));
    tool_registry.register(Arc::new(SearchCodeTool));
    let tools = Arc::new(tool_registry);

    let memory = match CompositeMemory::new(
        cfg.memory.working_memory_limit,
        std::path::Path::new(&cfg.memory.project_db_path),
        std::path::Path::new(&cfg.memory.user_db_path),
    ) {
        Ok(m) => Arc::new(tokio::sync::Mutex::new(m)),
        Err(e) => {
            eprintln!("{}: {}", t!("err_memory_init", msg = e.to_string()), e);
            return;
        }
    };

    let mut profiles = ProfileRegistry::new();
    if let Err(e) = profiles.load_profiles(std::path::Path::new("profiles")) {
        tracing::warn!("Failed to load profiles: {}", e);
    }
    let _profiles = Arc::new(tokio::sync::RwLock::new(profiles));

    let pool = Arc::new(SubagentPool::start(4));
    let orchestrator =
        Orchestrator::with_pool(llm.clone(), tools.clone(), events.clone(), pool.clone());
    let orchestrator = Arc::new(tokio::sync::Mutex::new(orchestrator));

    let mut concierge = Concierge::new(
        events.clone(),
        memory.clone(),
        orchestrator.clone(),
        llm.clone(),
        cfg.profiles.default.clone(),
    );

    if let Err(e) = register_slash_commands(&mut concierge) {
        eprintln!("{}: {}", t!("err_slash_register", msg = e.to_string()), e);
        std::process::exit(1);
    }

    let concierge = std::sync::Arc::new(tokio::sync::Mutex::new(concierge));

    println!("{}", t!("cli_interactive_banner"));

    let initial_profile = concierge.lock().await.active_profile().await;
    let initial_cache_hit_rate = "0/0".to_string();
    let tui = TuiBackend::with_initial(initial_profile, initial_cache_hit_rate);
    tui.run(concierge.clone(), events.clone());

    pool.shutdown().await;
    events.publish(Event::SystemShutdown);
}

/// v1.3.2 T7: 注册 6 个 builtin 斜杠命令到 Concierge
fn register_slash_commands(
    concierge: &mut Concierge,
) -> std::result::Result<(), qingbird_code::common::error::EflowError> {
    use qingbird_code::interaction::slash::CommandRegistry;
    use qingbird_code::interaction::slash::builtin::{
        help::HelpCmd, lang::LangCmd, level::LevelCmd, model::ModelCmd, profile::ProfileCmd,
        quit::QuitCmd,
    };

    let mut registry = CommandRegistry::new();
    registry.register(std::sync::Arc::new(ModelCmd));
    registry.register(std::sync::Arc::new(ProfileCmd));
    registry.register(std::sync::Arc::new(LangCmd));
    registry.register(std::sync::Arc::new(LevelCmd));
    registry.register(std::sync::Arc::new(HelpCmd::new(&registry)));
    registry.register(std::sync::Arc::new(QuitCmd));
    registry.required_register(&["model", "profile", "lang", "level", "help", "quit"])?;
    concierge.command_registry = registry;
    Ok(())
}
