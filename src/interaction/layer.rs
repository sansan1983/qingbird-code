//! v1.2 F2: InteractionLayer trait — 核心层与交互层解耦
//!
//! 设计 §14.1：核心层（Concierge / Orchestrator）通过 trait 与交互层（CLI / TUI / 未来 GUI）通信。
//! 当前 v1.1 完全没有 trait，main.rs 直接耦合 stdin 读行。
//! v1.2 抽 trait 让 TUI 和未来的 GUI 可以零侵入接入。

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::application::concierge::Concierge;
use crate::infrastructure::event::EventChannel;

/// 交互层抽象
///
/// `run` 同步阻塞直到 Quit / SystemShutdown，由 `main` 在 `#[tokio::main]` 上下文里
/// 直接同步调用（ratatui + crossterm 事件循环是同步的）。Concierge 是 `Arc<Mutex<>>` 包裹
/// 以便 `tokio::spawn` 异步派发任务（Concierge 内部已持有 `Orchestrator` 的 `Arc<Mutex<>>`）。
pub trait InteractionLayer: Send + Sync {
    fn run(&self, concierge: Arc<Mutex<Concierge>>, events: EventChannel);
}
