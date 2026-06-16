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
use eflow::interaction::cli::Cli;
use rust_i18n::t;

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "eflow=info".into()),
        )
        .init();

    let cli = Cli::parse_args();

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

    let llm = match LlmRouter::from_config(&cfg) {
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
    let concierge = Concierge::new(
        events.clone(),
        memory.clone(),
        profiles.clone(),
        orchestrator.clone(),
        cfg.profiles.default.clone(),
    );

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
        let ack = concierge.handle_input(task).await;
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

    // 交互模式
    println!("{}", t!("cli_interactive_banner"));

    // 后台监听事件
    let event_rx = events.clone();
    tokio::spawn(async move {
        let mut rx = event_rx.subscribe();
        loop {
            match rx.recv().await {
                Ok(
                    event @ (Event::TaskCompleted { .. }
                    | Event::TaskFailed { .. }
                    | Event::RiskEscalated { .. }),
                ) => print_event(&event),
                Ok(Event::SystemShutdown) => break,
                _ => {}
            }
        }
    });

    loop {
        // v1.0: 行输入模式（v1.2 升级为 TUI）
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let input = input.trim().to_string();
                if input.is_empty() {
                    continue;
                }
                if input == "quit" || input == "exit" {
                    break;
                }

                let response = concierge.handle_input(input).await;
                println!("> {response}");
            }
            Err(e) => {
                eprintln!("Input error: {e}");
                break;
            }
        }
    }

    // v1.1 M10.5 Task C6: 优雅关闭 SubagentPool（worker 退出）
    pool.shutdown().await;
    events.publish(Event::SystemShutdown);
}

/// 打印 CLI 事件（统一前缀处理，v1.0.3 R3 抽离）
fn print_event(event: &Event) {
    let line = match event {
        Event::TaskCompleted { task_id, summary } => format!(
            "{} [{}]: {}",
            t!("cli_task_completed"),
            short_id(task_id),
            summary.chars().take(200).collect::<String>()
        ),
        Event::TaskFailed { task_id, error } => format!(
            "{} [{}]: {}",
            t!("cli_task_failed"),
            short_id(task_id),
            error
        ),
        Event::RiskEscalated { task_id, from, to } => format!(
            "{} [{}]: {:?}→{:?}",
            t!("cli_risk_escalated"),
            short_id(task_id),
            from,
            to
        ),
        _ => return,
    };
    println!("\n{line}");
}

fn short_id(task_id: &uuid::Uuid) -> String {
    task_id.to_string().chars().take(8).collect()
}
