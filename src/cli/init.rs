//! `eflow init` —— 委托 spec B1 的 Wizard 状态机
//!
//! 退出码（契约冻结 v1.3.0 起 — spec B2 ADR-0017）：
//! - 0 = 成功（Wizard 跑完 7 步 + 写 config 成功）
//! - 1 = 用户 Esc 取消
//! - 1 或 2 = Wizard 内部错误（具体看 `error::exit_code` 映射）
//!
//! 设计：直接搬 v1.3.1 main.rs::run_init_wizard，**不**构造 LlmRouter
//! —— Wizard 走 stdin/stdout 填字段，**不**需要 router/Concierge。
//! v1.3.2 spec D 实施时把 stdin 改成 TUI（spec 写）

use crate::cli::error::exit_code;
use crate::cli::output::CliOutput;
use crate::common::error::Result;

/// 跑 init 向导，返回 i32 exit code（0/1/2）—— `std::process::exit()` 直接吃
pub fn run() -> i32 {
    use crate::interaction::wizard::Wizard;
    use crate::interaction::wizard::builtin::{
        apikey::ApikeyStep, confirm::ConfirmStep, language::LanguageStep, model::ModelStep,
        protocol::ProtocolStep, provider::ProviderStep, welcome::WelcomeStep,
    };

    let steps: Vec<std::sync::Arc<dyn crate::interaction::wizard::WizardStep>> = vec![
        std::sync::Arc::new(WelcomeStep),
        std::sync::Arc::new(LanguageStep),
        std::sync::Arc::new(ProviderStep),
        std::sync::Arc::new(ProtocolStep),
        std::sync::Arc::new(ApikeyStep),
        std::sync::Arc::new(ModelStep),
        std::sync::Arc::new(ConfirmStep),
    ];
    let wizard = Wizard::new(steps);
    match wizard.run() {
        Ok(crate::interaction::wizard::WizardOutcome::Completed(_state)) => {
            CliOutput::info("init complete; run `eflow` to start TUI");
            0
        }
        Ok(crate::interaction::wizard::WizardOutcome::Cancelled) => {
            CliOutput::info("init cancelled by user");
            1 // 用户主动 Esc — 用户错误
        }
        Err(e) => {
            CliOutput::error(&format!("init failed: {e}"));
            exit_code(&e)
        }
    }
}

// 保留 Result<()> import 以便未来扩展（如写文件 + 处理 IO 错误）
#[allow(dead_code)]
fn _r() -> Result<()> {
    Ok(())
}
