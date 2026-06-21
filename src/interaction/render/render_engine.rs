//! RenderEngine trait + DefaultRenderEngine（v1.4 唯一 impl）
//!
//! 把 ViewModel 翻译为 DrawCommand。颜色/前缀/边框**硬编码**在这里。
//! 未来 v1.5+ 可加 HighContrastEngine / DarkEngine 等替代 impl。

use super::draw_command::*;
use super::view_model::*;

/// 渲染引擎 trait（显卡）
pub trait RenderEngine: Send + Sync {
    /// 把 FrameViewModel 翻译为 DrawCommand 列表
    fn render(&self, vm: &FrameViewModel) -> Vec<DrawCommand>;
}

/// 默认实现：硬编码 cyan / red / yellow 配色 + ▶ 前缀 + Rounded 边框
pub struct DefaultRenderEngine {
    selected_prefix: String,
    unselected_prefix: String,
}

impl DefaultRenderEngine {
    pub fn new() -> Self {
        Self {
            selected_prefix: "▶ ".into(),
            unselected_prefix: "  ".into(),
        }
    }
}

impl Default for DefaultRenderEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderEngine for DefaultRenderEngine {
    fn render(&self, vm: &FrameViewModel) -> Vec<DrawCommand> {
        match vm {
            FrameViewModel::FullScreen(screen) => self.render_screen(screen),
            FrameViewModel::Modal { background, popup } => {
                let mut cmds = vec![DrawCommand::ClearArea];
                cmds.extend(self.render_screen(background));
                cmds.extend(self.render_screen(popup));
                cmds
            }
        }
    }
}

impl DefaultRenderEngine {
    fn render_screen(&self, vm: &ScreenViewModel) -> Vec<DrawCommand> {
        match vm {
            ScreenViewModel::Wizard(step) => self.render_step(step),
            ScreenViewModel::SelectList(list) => self.render_select_list(list),
            ScreenViewModel::Main(main) => self.render_main(main),
        }
    }

    fn render_step(&self, vm: &StepViewModel) -> Vec<DrawCommand> {
        let mut cmds = vec![DrawCommand::Block {
            border: BorderToken::Rounded,
            title: vm.title.clone(),
        }];
        for line in &vm.lines {
            cmds.push(DrawCommand::Text {
                content: line.text.clone(),
                style: TextStyle::default(),
            });
        }
        if let Some(input) = &vm.input {
            cmds.push(DrawCommand::Text {
                content: format!("{}: {}", input.label, input.value),
                style: if vm.focused_field == 0 {
                    TextStyle::primary()
                } else {
                    TextStyle::default()
                },
            });
        }
        for hint in &vm.hints {
            cmds.push(DrawCommand::Text {
                content: format!("[{}] {}", hint.key, hint.description),
                style: TextStyle::cache(),
            });
        }
        cmds
    }

    fn render_select_list(&self, vm: &SelectListViewModel) -> Vec<DrawCommand> {
        let mut cmds = vec![DrawCommand::Block {
            border: BorderToken::Rounded,
            title: vm.title.clone(),
        }];
        for (idx, item) in vm.items.iter().enumerate() {
            let prefix = if item.is_selected {
                &self.selected_prefix
            } else {
                &self.unselected_prefix
            };
            let style = if item.disabled {
                TextStyle::disabled()
            } else if item.is_selected {
                TextStyle::primary()
            } else {
                TextStyle::default()
            };
            cmds.push(DrawCommand::Span {
                prefix: prefix.clone(),
                content: format!("{}. {}", idx + 1, item.label),
                style,
            });
        }
        cmds
    }

    fn render_main(&self, vm: &MainViewModel) -> Vec<DrawCommand> {
        let mut cmds = Vec::new();

        // Header
        let mut header_spans = vec![DrawCommand::Text {
            content: format!("eflow | profile: {}", vm.header.profile),
            style: TextStyle::primary(),
        }];
        if !vm.header.configured {
            header_spans.push(DrawCommand::Text {
                content: " | ⚠ 未配置 LLM provider".into(),
                style: TextStyle::warning(),
            });
        }
        cmds.push(DrawCommand::Line {
            commands: header_spans,
        });

        // Messages
        for msg in &vm.messages {
            cmds.push(DrawCommand::Text {
                content: msg.text.clone(),
                style: TextStyle::default(),
            });
        }

        // Status
        cmds.push(DrawCommand::Text {
            content: vm.status.clone(),
            style: TextStyle::cache(),
        });

        // Prompt
        cmds.push(DrawCommand::Text {
            content: format!("> {}", vm.prompt),
            style: TextStyle::default(),
        });

        cmds
    }
}
