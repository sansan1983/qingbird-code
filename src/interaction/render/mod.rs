//! v1.4 spec D: 渲染管线（电脑主机架构）
//!
//! 三层架构：
//! - 核心层（wizard / SelectList / TuiBackend 4 段布局）输出 `ViewModel` 纯数据
//! - [`RenderEngine`]（显卡）翻译 ViewModel → `DrawCommand`（硬编码颜色/前缀/边框）
//! - [`RenderBackend`]（驱动）机械执行 DrawCommand 画到屏幕
//!
//! 4 条铁律：
//! 1. 核心层零 ratatui import
//! 2. RenderBackend 零业务知识
//! 3. Modal 走完整路径
//! 4. TuiBackend::run() 不准业务判断
//!
//! ADR-0017: 核心零硬编码渲染

pub mod draw_command;
pub mod render_backend;
pub mod render_engine;
pub mod view_model;

pub use draw_command::*;
pub use render_backend::*;
pub use render_engine::*;
pub use view_model::*;

#[cfg(test)]
mod view_model_test;

#[cfg(test)]
mod render_engine_test;
