//! help handler —— 列可用 slash commands
//!
//! 设计：直接调 `command_registry.list()` —— 输出 (name, help) 对，
//! GUI 可以渲染为可点击列表。v1.3.1 HelpCmd::execute 已做类似输出，handler 复用
//! registry API 避免走 SlashCommand 链路。
use crate::application::concierge::Concierge;
use crate::cli::output::CliOutput;
use crate::common::error::Result;

pub async fn dispatch(concierge: &mut Concierge) -> Result<()> {
    let cmds = concierge.command_registry.list();
    let mut text = String::from("可用命令:\n");
    for (name, help) in &cmds {
        text.push_str(&format!("  /{:<12} {}\n", name, help));
    }
    CliOutput::info(&text);
    Ok(())
}
