//! v1.3.2 headless CLI 契约层
//!
//! 给 v2.0 GUI 套壳用。TUI 不走这里（spec B2 ADR-0016）。
//!
//! 契约冻结 v1.3.0 起（spec B2 ADR-0017）：
//! - stdout 永远 JSON 契约
//! - stderr 永远人类可读
//! - 7 个事件 schema 不变
//! - 5 个 stdin action schema 不变
//! - 4 档 exit code 不变

pub mod config;
pub mod error;
pub mod handlers;
pub mod init;
pub mod output;
pub mod prompt;
pub mod start;
pub mod stdin;
