use clap::{Parser, Subcommand};

/// eflow — Efficient Flow Agent Framework
#[derive(Parser, Debug)]
#[command(name = "eflow")]
#[command(version = "0.1.0")]
#[command(about = "Multi-layer Agent Collaboration Framework", long_about = None)]
pub struct Cli {
    /// 子命令
    #[command(subcommand)]
    pub command: Option<Command>,

    /// 直接执行单次任务（非交互模式）
    #[arg(short, long)]
    pub execute: Option<String>,

    /// 显示配置
    #[arg(long)]
    pub show_config: bool,

    /// 列出可用 Profile
    #[arg(long)]
    pub list_profiles: bool,

    /// 覆盖 locale（启动时优先于 config.core.language）
    #[arg(long)]
    pub lang: Option<String>,
}

/// eflow 子命令
#[derive(Subcommand, Debug)]
pub enum Command {
    /// 运行配置向导
    Init,
    /// 启动 TUI 对话模式
    Tui,
    /// Session 管理
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
}

/// eflow session 子命令
#[derive(Subcommand, Debug)]
pub enum SessionAction {
    /// 启动 headless 持续运行模式（NDJSON stdio 契约）
    Start {
        /// 配置路径
        #[arg(long)]
        config: Option<String>,
        /// 语言
        #[arg(long)]
        lang: Option<String>,
    },
}

impl Cli {
    #[must_use]
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
