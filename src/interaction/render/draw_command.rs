//! DrawCommand enum（RenderEngine 产物）
//!
//! 5 类指令覆盖全部画法。`TextStyle` 内部直接用 `ratatui::style::Color`
//! —— **本文件是 render/ 目录内唯一允许 import ratatui style 的文件**。

use ratatui::style::Color;

/// 边框风格 token
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderToken {
    Rounded,
    Square,
    Double,
    None,
}

/// 文本样式（硬编码颜色 token）
#[derive(Debug, Clone, Copy, Default)]
pub struct TextStyle {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
}

impl TextStyle {
    /// 主色（cyan）— 选中项 / 重要信息
    pub fn primary() -> Self {
        Self {
            fg: Some(Color::Cyan),
            ..Default::default()
        }
    }

    /// 警告色（red）— 错误 / 未配置
    pub fn warning() -> Self {
        Self {
            fg: Some(Color::Red),
            ..Default::default()
        }
    }

    /// 缓存色（yellow）— 状态指示
    pub fn cache() -> Self {
        Self {
            fg: Some(Color::Yellow),
            ..Default::default()
        }
    }

    /// 灰显（DarkGray）— disabled 项
    pub fn disabled() -> Self {
        Self {
            fg: Some(Color::DarkGray),
            ..Default::default()
        }
    }
}

/// 渲染指令（RenderEngine 输出）
#[derive(Debug, Clone)]
pub enum DrawCommand {
    /// 单行文本
    Text { content: String, style: TextStyle },
    /// 带边框 + 标题
    Block { border: BorderToken, title: String },
    /// "▶ Item 1" 格式（带前缀）
    Span {
        prefix: String,
        content: String,
        style: TextStyle,
    },
    /// 多 span 拼一行
    Line { commands: Vec<DrawCommand> },
    /// 清空区域
    ClearArea,
}
