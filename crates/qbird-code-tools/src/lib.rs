//! qbird-code-tools — 内置工具（读文件、写文件、执行命令、搜索代码）
//!
//! 本 crate 实现了 4 个内置工具，每个工具实现了 `Tool` trait，
//! 并通过 `ToolRegistry` 统一管理。
//!
//! # i18n
//!
//! 本 crate 使用 `rust_i18n` 进行国际化，`i18n!` 宏指向 workspace 根目录的
//! `locales/` 目录。路径 `../../locales` 从本 crate 的根目录（`Cargo.toml` 所在目录）
//! 出发，相对于 workspace 根目录。

rust_i18n::i18n!("../../locales", fallback = "en-US");

pub mod command;
pub mod file;
pub mod glob;
pub mod registry;
pub mod search;

pub use command::ExecuteCommandTool;
pub use file::{ReadFileTool, WriteFileTool};
pub use glob::{GlobTool, glob_match};
pub use registry::{Tool, ToolDefinition, ToolOutput, ToolRegistry};
pub use search::SearchCodeTool;
