//! `qingbird session start` —— 持续运行的 headless 模式
//!
//! 启动流程：
//! 1. 加载 config
//! 2. 切语言（如果指定）
//! 3. 构造 LlmRouter + tools + orchestrator + pool + Concierge
//! 4. 注册 6 个 builtin 斜杠命令
//! 5. 订阅事件通道
//! 6. 输出 SystemReady JSON 事件（**第一行**）
//! 7. tokio::select 跑 2 个 task：
//!    - 任务 A：监听 event channel → 6 个事件 → CliOutput::ndjson_event()
//!    - 任务 B：读 stdin → 5 个 StdinCommand → handler
//! 8. Ctrl+C / EOF / SystemShutdown → 退出码
//!
//! 关键设计决策（spec B2 ADR-0018）：
//! - 单 subcommand 模式——启动即 headless，不回到 TUI
//! - SystemReady 为第一行 stdout——让 GUI 确认启动成功
//! - TUI 零改造（ADR-0016）——TUI 仍走 main.rs 交互层

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::broadcast;
use uuid::Uuid;

use crate::application::concierge::Concierge;
use crate::application::orchestrator::Orchestrator;
use crate::capability::pool::SubagentPool;
use crate::capability::tools::{
    ToolRegistry,
    command::ExecuteCommandTool,
    file::{ReadFileTool, WriteFileTool},
    search::SearchCodeTool,
};
use crate::infrastructure::config;
use crate::infrastructure::event::Event;
use crate::infrastructure::event::EventChannel;
use crate::infrastructure::llm::LlmRouter;
use crate::infrastructure::memory::CompositeMemory;
use crate::interaction::slash::CommandRegistry;
use crate::interaction::slash::builtin::{
    help::HelpCmd, lang::LangCmd, level::LevelCmd, model::ModelCmd, profile::ProfileCmd,
    quit::QuitCmd,
};

use super::error::exit_code;
use super::output::CliOutput;

/// 启动会话并持续运行
///
/// 返回 i32 exit code（0/1/2/130）—— `std::process::exit()` 直接吃
pub async fn run(config_path: Option<PathBuf>, lang: Option<String>) -> i32 {
    // 1. 加载 config
    let config_path = config_path.unwrap_or_else(|| {
        dirs::config_dir()
            .map(|p| p.join("eflow").join("config.yaml"))
            .unwrap_or_else(|| PathBuf::from("./eflow.yaml"))
    });
    let eflow_config = match config::load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            CliOutput::error(&format!("config load failed: {e}"));
            return exit_code(&e);
        }
    };

    // 2. 切语言（如��指定）
    let lang_str = lang
        .as_deref()
        .or(Some(eflow_config.core.language.as_str()));
    crate::infrastructure::locale::init(lang_str);

    // 3. 初始化基础设施
    let events = EventChannel::new();

    let llm_router = match LlmRouter::from_config(&eflow_config) {
        Ok(r) => Arc::new(tokio::sync::Mutex::new(r)),
        Err(e) => {
            CliOutput::error(&format!("LLM router init failed: {e}"));
            return exit_code(&e);
        }
    };

    // 4. 初始化工具注册表
    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(Arc::new(ReadFileTool));
    tool_registry.register(Arc::new(WriteFileTool));
    tool_registry.register(Arc::new(ExecuteCommandTool));
    tool_registry.register(Arc::new(SearchCodeTool));
    let tools = Arc::new(tool_registry);

    // 5. 初始化记忆
    let memory = match CompositeMemory::new(
        eflow_config.memory.working_memory_limit,
        std::path::Path::new(&eflow_config.memory.project_db_path),
        std::path::Path::new(&eflow_config.memory.user_db_path),
    ) {
        Ok(m) => Arc::new(tokio::sync::Mutex::new(m)),
        Err(e) => {
            CliOutput::error(&format!("memory init failed: {e}"));
            return exit_code(&e);
        }
    };

    // 6. 启动 SubagentPool
    let pool = Arc::new(SubagentPool::start(4));

    // 7. 构造 Orchestrator
    let orchestrator = Orchestrator::with_pool(
        llm_router.clone(),
        tools.clone(),
        events.clone(),
        pool.clone(),
    );
    let orchestrator = Arc::new(tokio::sync::Mutex::new(orchestrator));

    // 8. 构造 Concierge
    let mut concierge = Concierge::new(
        events.clone(),
        memory,
        orchestrator,
        llm_router,
        eflow_config.profiles.default.clone(),
    );

    // 9. 注册 6 个 builtin 斜杠命令（与 main.rs 一致）
    let mut registry = CommandRegistry::new();
    registry.register(Arc::new(ModelCmd));
    registry.register(Arc::new(ProfileCmd));
    registry.register(Arc::new(LangCmd));
    registry.register(Arc::new(LevelCmd));
    registry.register(Arc::new(HelpCmd::new(&registry)));
    registry.register(Arc::new(QuitCmd));
    if let Err(e) =
        registry.required_register(&["model", "profile", "lang", "level", "help", "quit"])
    {
        CliOutput::error(&format!("command registry init failed: {e}"));
        return exit_code(&e);
    }
    concierge.command_registry = registry;

    // 10. 订阅事件通道（Concierge 暴露 getter）
    let mut event_rx = concierge.subscribe_events();

    // 11. 输出 SystemReady（**第一行** stdout）
    if let Err(e) = CliOutput::ndjson_event(&serde_json::json!({
        "event_type": "SystemReady",
        "task_id": Uuid::nil(),
        "started_at": chrono::Utc::now().to_rfc3339(),
    })) {
        return exit_code(&crate::common::error::EflowError::Serialization(
            e.to_string(),
        ));
    }

    // 12. tokio::select 跑 2 个 task
    let code = run_two_tasks(&mut concierge, &mut event_rx, &pool, &events).await;

    // 13. 优雅退出
    pool.shutdown().await;
    events.publish(Event::SystemShutdown);
    code
}

async fn run_two_tasks(
    concierge: &mut Concierge,
    event_rx: &mut broadcast::Receiver<Event>,
    _pool: &Arc<SubagentPool>,
    _events: &EventChannel,
) -> i32 {
    tokio::select! {
        code = task_a_listen_events(event_rx) => code,
        code = super::stdin::read_loop(concierge) => code,
        _ = tokio::signal::ctrl_c() => super::error::handle_sigint(),
    }
}

async fn task_a_listen_events(event_rx: &mut broadcast::Receiver<Event>) -> i32 {
    loop {
        match event_rx.recv().await {
            Ok(Event::TaskStarted {
                task_id,
                description,
            }) => {
                let _ = CliOutput::ndjson_event(&serde_json::json!({
                    "event_type": "TaskStarted",
                    "task_id": task_id,
                    "description": description,
                }));
            }
            Ok(Event::TaskCompleted { task_id, summary }) => {
                let _ = CliOutput::ndjson_event(&serde_json::json!({
                    "event_type": "TaskCompleted",
                    "task_id": task_id,
                    "summary": summary,
                }));
            }
            Ok(Event::TaskFailed { task_id, error }) => {
                let _ = CliOutput::ndjson_event(&serde_json::json!({
                    "event_type": "TaskFailed",
                    "task_id": task_id,
                    "error": error,
                }));
            }
            Ok(Event::RiskEscalated { task_id, from, to }) => {
                let _ = CliOutput::ndjson_event(&serde_json::json!({
                    "event_type": "RiskEscalated",
                    "task_id": task_id,
                    "from": format!("{:?}", from),
                    "to": format!("{:?}", to),
                }));
            }
            Ok(Event::UserInputRequired { prompt }) => {
                let _ = CliOutput::ndjson_event(&serde_json::json!({
                    "event_type": "UserInputRequired",
                    "prompt": prompt,
                }));
            }
            Ok(Event::SystemShutdown) => {
                return 0;
            }
            // v1.3.2: SystemReady 当前未在 channel 流通（start.rs 手写 NDJSON），
            // 保留分支以便将来按事件流分发时复用
            Ok(Event::SystemReady { .. }) => continue,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => return 0,
        }
    }
}
