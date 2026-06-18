use std::sync::Arc;

// 在 bin crate 中也调用 i18n!() 以生成 _rust_i18n_t! 宏（让 main.rs 里的 t!() 可用）
rust_i18n::i18n!("locales", fallback = "en-US");

use eflow::application::concierge::Concierge;
use eflow::application::orchestrator::Orchestrator;
use eflow::capability::pool::SubagentPool;
use eflow::capability::tools::{
    ToolRegistry,
    command::ExecuteCommandTool,
    file::{ReadFileTool, WriteFileTool},
    search::SearchCodeTool,
};
use eflow::common::types::ModelTier;
use eflow::infrastructure::config;
use eflow::infrastructure::event::{Event, EventChannel};
use eflow::infrastructure::llm::LlmRouter;
use eflow::infrastructure::locale;
use eflow::infrastructure::memory::CompositeMemory;
use eflow::infrastructure::profile::ProfileRegistry;
use eflow::interaction::InteractionLayer;
use eflow::interaction::cli::Cli;
use eflow::interaction::tui::TuiBackend;
use rust_i18n::t;

#[tokio::main]
async fn main() {
    // 初始化日志（走 stderr —— 契约冻结 v1.3.0 起：stdout 永远 JSON 契约，
    // stderr 永远人类可读。spec B2 §3.5 ADR-0017）
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "eflow=info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    // v1.3.2 T7: `eflow init` 子命令委托 cli::init（v1.3.1 main.rs 也有 init 路由——cli/init.rs 是 surgical 搬移）
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("init") {
        let exit_code = eflow::cli::init::run();
        std::process::exit(exit_code);
    }

    // v1.3.2 T1: `eflow session start` 子命令——headless 持续运行（spec B2）
    // plan deviation #12f：v1.3.1 main.rs 没有 clap SubCommand enum，
    // 用 std::env::args() 手工切路由，匹配 v1.3.1 既有 init 子命令风格。
    if args.get(1).map(String::as_str) == Some("session") {
        // 用法：eflow session start [--config PATH] [--lang LANG]
        match args.get(2).map(String::as_str) {
            Some("start") => {
                let config = parse_session_flag(&args, "--config").map(std::path::PathBuf::from);
                let lang = parse_session_flag(&args, "--lang");
                let exit_code = eflow::cli::start::run(config, lang).await;
                std::process::exit(exit_code);
            }
            Some(other) => {
                eprintln!(
                    "未知 session 子命令: {other}。用法: eflow session start [--config PATH] [--lang LANG]"
                );
                std::process::exit(1);
            }
            None => {
                eprintln!(
                    "缺少 session 子命令。用法: eflow session start [--config PATH] [--lang LANG]"
                );
                std::process::exit(1);
            }
        }
    }

    let cli = Cli::parse_args();

    // v1.3.1 T10: 首次启动检测——配置不存在时,提示是否进 init 向导
    if !cli.show_config && !cli.list_profiles && cli.execute.is_none() {
        // 只在交互式启动时检测(--show-config / --execute / --list-profiles 不挡 wizard)
        if let Some(path) = config::find_config()
            && !path.exists()
        {
            use std::io::BufRead;
            eprintln!("未找到配置 ({})", path.display());
            eprint!("是否进入初始化向导？[Y/n] ");
            let mut line = String::new();
            let _ = std::io::stdin().lock().read_line(&mut line);
            let line = line.trim().to_lowercase();
            if !line.starts_with('n') {
                let code = eflow::cli::init::run();
                if code != 0 {
                    std::process::exit(code);
                }
                eprintln!("配置已写入。运行 eflow 启动 TUI。");
                return;
            }
            // 用户选 N → 继续往下走,会因配置缺失而 exit
        }
    }

    // 加载配置
    let config_path = config::find_config().unwrap_or_else(|| {
        eprintln!("{}", t!("cli_no_config"));
        std::path::PathBuf::from("eflow.yaml")
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
        .map(|p| p.join("eflow").join("providers"))
        .unwrap_or_else(|| std::path::PathBuf::from("./providers"));
    let _ = std::fs::create_dir_all(&provider_dir); // 不存在就建（首次启动友好）
    let llm = match LlmRouter::from_config(&cfg, &provider_dir) {
        Ok(l) => Arc::new(tokio::sync::Mutex::new(l)),
        Err(e) => {
            eprintln!("Failed to init LLM: {e}");
            return;
        }
    };

    // v1.1 Task B7: 启动时打印 L2 状态（设计 §8.5）
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
            eprintln!("Failed to init memory: {e}");
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
    // v1.1 M10.5 Task C6: 启动 SubagentPool 注入 Orchestrator
    let pool = Arc::new(SubagentPool::start(4));
    let orchestrator =
        Orchestrator::with_pool(llm.clone(), tools.clone(), events.clone(), pool.clone());
    let orchestrator = Arc::new(tokio::sync::Mutex::new(orchestrator));

    // 初始化 Concierge
    let mut concierge = Concierge::new(
        events.clone(),
        memory.clone(),
        profiles.clone(),
        orchestrator.clone(),
        llm.clone(), // v1.3.1 增量
        cfg.profiles.default.clone(),
    );

    // v1.3.1 T10: 注册 6 个斜杠命令 + required_register 校验
    if let Err(e) = register_slash_commands(&mut concierge) {
        eprintln!("斜杠命令注册失败: {e}");
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

    // 单次执行模式（v1.1 e2e: 等异步 TaskCompleted 事件，让 CLI 有真正 \"fire-then-wait\" 语义）
    if let Some(task) = cli.execute {
        // subscribe 必须早于 handle_input，否则 race：超快 LLM 响应可能在 subscribe 之前 fire
        let mut rx = events.subscribe();
        let ack = concierge.lock().await.handle_input(task).await;
        println!("{ack}");

        // 等 TaskCompleted / TaskFailed，60s timeout 防挂死
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
        return;
    }

    // 交互模式（TUI — 设计 §14.3）
    // v1.2 F6: 默认走 TUI，--execute / --show-config / --list-profiles 保留 CLI 直输出
    println!("{}", t!("cli_interactive_banner"));

    // TUI 模式：先在 async 上下文填好 profile + cache stats（F3 with_initial）
    // —— TuiBackend::run 是 sync，profile/cache_hit_rate 必须在 run() 前准备好
    let initial_profile = concierge.lock().await.active_profile().await;
    let initial_cache_hit_rate = "0/0".to_string(); // v1.2 占位：v1.3 接 L2 cache stats
    let tui = TuiBackend::with_initial(initial_profile, initial_cache_hit_rate);
    tui.run(concierge.clone(), events.clone());

    // v1.1 M10.5 Task C6: 优雅关闭 SubagentPool（worker 退出）
    pool.shutdown().await;
    events.publish(Event::SystemShutdown);
}

/// v1.3.2 T7: 注册 6 个 builtin 斜杠命令到 Concierge
fn register_slash_commands(
    concierge: &mut Concierge,
) -> std::result::Result<(), eflow::common::error::EflowError> {
    use eflow::interaction::slash::CommandRegistry;
    use eflow::interaction::slash::builtin::{
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

/// v1.3.2 T1: 解析 `eflow session start --flag VALUE` 的 flag 值
///
/// # 行为
/// - 从 args 找 `--flag`，下一个 arg 当 value
/// - flag 存在但无 value → 视为 None（让 start.rs 用默认）
/// - flag 不存在 → None
///
/// # 简单实现理由
/// - v1.3.2 spec B2 阶段只用 `--config` / `--lang` 两个 flag
/// - 等 v1.4 spec D 引入 clap derive 时替换
fn parse_session_flag(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == flag {
            return iter.next().cloned();
        }
    }
    None
}
