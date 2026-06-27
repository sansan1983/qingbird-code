rust_i18n::i18n!("../../locales", fallback = "en-US");

pub mod doom_loop;
pub mod nudge;
pub mod react_loop;
pub mod skill;
pub mod subagent;
pub mod subagent_pool;

pub use react_loop::{ReactLoop, ReactLoopConfig};
pub use subagent::{Subagent, SubagentConfig, SubagentRole};
pub use subagent_pool::execute_parallel;
