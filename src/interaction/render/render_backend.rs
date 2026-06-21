//! RenderBackend trait（显卡驱动）
//!
//! 机械执行 DrawCommand 画到屏幕。**零业务知识**——不知道是 wizard 还是
//! SelectList，只知道「画 DrawCommand」。ratatui 实现见 `tui.rs`。

use ratatui::layout::Rect;
use ratatui::prelude::Buffer;

use super::draw_command::DrawCommand;

/// 渲染后端 trait
pub trait RenderBackend {
    /// 执行 DrawCommand 列表到指定 area
    fn execute(&mut self, commands: &[DrawCommand], area: Rect, buf: &mut Buffer);
}
