//! Subagent 系统：让 LLM 主动派发子任务给独立 ReAct 循环实例。
//!
//! 设计参考 `F:\AI\Kun\kun\src\delegation/`：
//! - `profile.rs`    — `SubagentProfile` 数据模型 + 内置字典
//! - `config.rs`     — yaml 加载 + 与内置合并
//! - `executor.rs`   — 子 agent 生命周期管理（PR-B2）
//!
//! 这个模块是 v0.4 进化系统（CompactionManager / Reflection Engine /
//! Profile Compilation）的通用管道；预留 `model` 字段和 `SubagentSpawnHints`
//! 让这些 feature 接入时零摩擦。

pub mod config;
pub mod executor;
pub mod profile;

pub use config::{SubagentProfileConfig, load_profiles};
pub use executor::{
    ChildEvent, ChildRecord, ChildStatus, SpawnPriority, SubagentExecutor, SubagentSpawnHints,
};
pub use profile::{SubagentMode, SubagentProfile, ToolPolicy, builtin_profiles};
