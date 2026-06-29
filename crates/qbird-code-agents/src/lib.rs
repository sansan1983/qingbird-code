rust_i18n::i18n!("../../locales", fallback = "en-US");

pub mod doom_loop;
pub mod nudge;
pub mod react_loop;
pub mod skill;
pub mod subagent;

pub use react_loop::{ReactLoop, ReactLoopConfig};
