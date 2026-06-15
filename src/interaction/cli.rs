use clap::Parser;

/// eflow — Efficient Flow Agent Framework
#[derive(Parser, Debug)]
#[command(name = "eflow")]
#[command(version = "0.1.0")]
#[command(about = "Multi-layer Agent Collaboration Framework", long_about = None)]
pub struct Cli {
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

impl Cli {
    #[must_use]
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
