//! 5 个 ViewModel struct/enum 定义
//!
//! 纯数据（无方法或仅纯函数）。**不包含坐标 / area 信息**——area 由
//! RenderBackend 在 execute 时从 frame 获取，ViewModel 只表达「显示什么」
//! 不表达「在哪里显示」。

/// 整屏视图（核心层产物）
#[derive(Debug, Clone)]
pub enum FrameViewModel {
    /// 全屏显示一种内容
    FullScreen(ScreenViewModel),
    /// 主屏 + 弹窗叠合
    Modal {
        background: ScreenViewModel,
        popup: ScreenViewModel,
    },
}

/// 屏幕内「一段」内容
#[derive(Debug, Clone)]
pub enum ScreenViewModel {
    /// wizard 步骤
    Wizard(StepViewModel),
    /// 列表 widget
    SelectList(SelectListViewModel),
    /// TUI 主屏 4 段布局
    Main(MainViewModel),
}

/// wizard 一步的视图
#[derive(Debug, Clone)]
pub struct StepViewModel {
    pub title: String,
    pub lines: Vec<LineVM>,
    pub input: Option<InputFieldVM>,
    pub hints: Vec<KeyHint>,
    pub focused_field: usize,
}

/// 列表 widget 的视图
#[derive(Debug, Clone)]
pub struct SelectListViewModel {
    pub title: String,
    pub items: Vec<ListItemVM>,
    pub selected: usize,
    pub scroll_offset: usize,
}

/// TUI 主屏 4 段布局
#[derive(Debug, Clone)]
pub struct MainViewModel {
    pub header: HeaderVM,
    pub messages: Vec<MessageVM>,
    pub status: String,
    pub prompt: String,
}

/// 一行文本
#[derive(Debug, Clone, Default)]
pub struct LineVM {
    pub text: String,
}

/// 输入框
#[derive(Debug, Clone)]
pub struct InputFieldVM {
    pub label: String,
    pub value: String,
    pub cursor_pos: usize,
}

/// 按键提示
#[derive(Debug, Clone)]
pub struct KeyHint {
    pub key: String,
    pub description: String,
}

/// 列表项
#[derive(Debug, Clone)]
pub struct ListItemVM {
    pub label: String,
    pub hint: Option<String>,
    pub disabled: bool,
    pub is_selected: bool,
}

/// Header 段
#[derive(Debug, Clone)]
pub struct HeaderVM {
    pub profile: String,
    pub cache_hit_rate: String,
    pub configured: bool,
}

/// 一条消息
#[derive(Debug, Clone)]
pub struct MessageVM {
    pub text: String,
}
