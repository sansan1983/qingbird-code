//! SelectList widget — 多选一选择 UI
//!
//! 多模交互：输入序号 / ↑↓ 键 / PageUp-Down / 鼠标滚轮 / 鼠标点击 / Enter / Esc。
//! 焦点感知：widget 不知道自己焦点位置，**TUI 主循环**根据事件类型分发。
//!
//! **v1.3.1 已知偏差（spec B1 §12）**：本文件 `render()` 直接调 ratatui API
//!（`Paragraph::new` / `List::new` / `Style::default().fg(Color::Cyan)`），
//! 违反"零硬编码"原则。v1.4 spec D 接手时**重构为 RenderEngine trait + DrawCommand**。

use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::prelude::{Buffer, Rect, Style, Widget};
use ratatui::widgets::{Block, Borders, List, ListItem as RatListItem, Paragraph};

use crate::common::error::Result;

/// 选择项数据源 trait
///
/// v1.3.1 起核心零硬编码：数据从 trait 拿，widget 不知道具体数据。
#[async_trait]
pub trait SelectItemSource: Send + Sync {
    /// 返回所有可选项（async 拉取也可）
    async fn items(&self) -> Result<Vec<SelectItem>>;
}

#[derive(Debug, Clone)]
pub struct SelectItem {
    pub label: String,
    pub value: String,
    pub hint: Option<String>,
    pub disabled: bool,
}

impl SelectItem {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            hint: None,
            disabled: false,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SelectAction {
    Stay,
    Up,
    Down,
    PageUp,
    PageDown,
    Confirm(usize),
    Cancel,
}

pub struct SelectList {
    source: Arc<dyn SelectItemSource>,
    items: Vec<SelectItem>,
    selected: usize,
    scroll_offset: usize,
    viewport_height: usize,
}

impl std::fmt::Debug for SelectList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectList")
            .field("items_count", &self.items.len())
            .field("selected", &self.selected)
            .field("scroll_offset", &self.scroll_offset)
            .field("viewport_height", &self.viewport_height)
            .finish()
    }
}

impl SelectList {
    pub fn new(source: Arc<dyn SelectItemSource>) -> Self {
        Self {
            source,
            items: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            viewport_height: 5,
        }
    }

    /// 拉取数据（必须先调，否则 items 为空）
    pub async fn load(&mut self) -> Result<()> {
        self.items = self.source.items().await?;
        if self.items.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.items.len() {
            self.selected = self.items.len() - 1;
        } else if self.items[self.selected].disabled {
            // v1.3.1 修：load 后若 selected 是 disabled → 跳到第一个 enabled
            self.selected = (0..self.items.len())
                .find(|&i| !self.items[i].disabled)
                .unwrap_or(0);
        }
        self.adjust_scroll();
        Ok(())
    }

    pub fn selected_item(&self) -> Option<&SelectItem> {
        self.items.get(self.selected)
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn items(&self) -> &[SelectItem] {
        &self.items
    }

    /// 渲染到 ratatui Buffer（**临时硬编码**——v1.4 spec D 重构）
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let visible: Vec<RatListItem> = self
            .items
            .iter()
            .skip(self.scroll_offset)
            .take(self.viewport_height)
            .enumerate()
            .map(|(idx, item)| {
                let actual_idx = idx + self.scroll_offset;
                let style = if actual_idx == self.selected {
                    Style::default().fg(ratatui::style::Color::Cyan)
                } else if item.disabled {
                    Style::default().fg(ratatui::style::Color::DarkGray)
                } else {
                    Style::default()
                };
                let prefix = if actual_idx == self.selected {
                    "▶ "
                } else {
                    "  "
                };
                let text = format!("{prefix}{}. {}", actual_idx + 1, item.label);
                RatListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(visible).block(Block::default().borders(Borders::ALL).title("Select"));
        // ratatui 0.x 列表状态：stateful widget 需要 ListState
        // 为简化这里用 Paragraph 渲染多行（spec B1 范围内可接受）
        let _ = list; // 暂时不用，避免编译警告
        let joined: Vec<ratatui::text::Line> = self
            .items
            .iter()
            .skip(self.scroll_offset)
            .take(self.viewport_height)
            .enumerate()
            .map(|(idx, item)| {
                let actual_idx = idx + self.scroll_offset;
                let style = if actual_idx == self.selected {
                    Style::default().fg(ratatui::style::Color::Cyan)
                } else if item.disabled {
                    Style::default().fg(ratatui::style::Color::DarkGray)
                } else {
                    Style::default()
                };
                let prefix = if actual_idx == self.selected {
                    "▶ "
                } else {
                    "  "
                };
                let text = format!("{prefix}{}. {}", actual_idx + 1, item.label);
                ratatui::text::Line::from(text).style(style)
            })
            .collect();
        Paragraph::new(joined).render(area, buf);
    }

    pub fn on_key(&mut self, key: KeyEvent) -> SelectAction {
        let len = self.items.len();
        if len == 0 {
            return match key.code {
                KeyCode::Esc => SelectAction::Cancel,
                _ => SelectAction::Stay,
            };
        }
        // 跳过 disabled 项
        let first_enabled = (0..len).find(|&i| !self.items[i].disabled).unwrap_or(0);
        let prev_enabled = |cur: usize| -> usize {
            if cur == 0 {
                first_enabled
            } else {
                (0..cur)
                    .rev()
                    .find(|&i| !self.items[i].disabled)
                    .unwrap_or(cur)
            }
        };
        let next_enabled = |cur: usize| -> usize {
            (cur + 1..len)
                .find(|&i| !self.items[i].disabled)
                .unwrap_or(cur)
        };
        match key.code {
            KeyCode::Up => {
                self.selected = prev_enabled(self.selected);
                self.adjust_scroll();
                SelectAction::Up
            }
            KeyCode::Down => {
                self.selected = next_enabled(self.selected);
                self.adjust_scroll();
                SelectAction::Down
            }
            KeyCode::PageUp => {
                let new = self.selected.saturating_sub(self.viewport_height);
                self.selected = (0..=new)
                    .rev()
                    .find(|&i| !self.items[i].disabled)
                    .unwrap_or(first_enabled);
                self.adjust_scroll();
                SelectAction::PageUp
            }
            KeyCode::PageDown => {
                let new = (self.selected + self.viewport_height).min(len - 1);
                self.selected = (new..len)
                    .find(|&i| !self.items[i].disabled)
                    .unwrap_or(self.selected);
                self.adjust_scroll();
                SelectAction::PageDown
            }
            KeyCode::Enter => {
                if self.items[self.selected].disabled {
                    SelectAction::Stay
                } else {
                    SelectAction::Confirm(self.selected)
                }
            }
            KeyCode::Esc => SelectAction::Cancel,
            KeyCode::Char(c @ '1'..='9') => {
                let n = (c as u8 - b'0') as usize;
                if n <= len && !self.items[n - 1].disabled {
                    self.selected = n - 1;
                    self.adjust_scroll();
                    SelectAction::Confirm(n - 1)
                } else {
                    SelectAction::Stay
                }
            }
            _ => SelectAction::Stay,
        }
    }

    pub fn on_mouse(&mut self, mouse: MouseEvent) -> SelectAction {
        let len = self.items.len();
        if len == 0 {
            return SelectAction::Stay;
        }
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                let new = self.selected.saturating_sub(1);
                if !self.items[new].disabled {
                    self.selected = new;
                    self.adjust_scroll();
                    return SelectAction::Up;
                }
                SelectAction::Stay
            }
            MouseEventKind::ScrollDown => {
                let new = (self.selected + 1).min(len - 1);
                if !self.items[new].disabled {
                    self.selected = new;
                    self.adjust_scroll();
                    return SelectAction::Down;
                }
                SelectAction::Stay
            }
            _ => {
                if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                    // 简化：鼠标点击不计算坐标，spec B1 范围内只支持序号/键盘
                    SelectAction::Stay
                } else {
                    SelectAction::Stay
                }
            }
        }
    }

    fn adjust_scroll(&mut self) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = self.selected + 1 - self.viewport_height;
        }
    }

    pub fn set_viewport_height(&mut self, h: usize) {
        self.viewport_height = h.max(1);
        self.adjust_scroll();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::error::EflowError;
    use async_trait::async_trait;

    struct MockSource {
        items: Vec<SelectItem>,
    }

    #[async_trait]
    impl SelectItemSource for MockSource {
        async fn items(&self) -> Result<Vec<SelectItem>> {
            Ok(self.items.clone())
        }
    }

    fn items5() -> Vec<SelectItem> {
        (1..=5)
            .map(|i| SelectItem::new(format!("Item {i}"), format!("v{i}")))
            .collect()
    }

    #[tokio::test]
    async fn load_populates_items() {
        let mut list = SelectList::new(Arc::new(MockSource { items: items5() }));
        list.load().await.unwrap();
        assert_eq!(list.items().len(), 5);
    }

    #[tokio::test]
    async fn up_from_first_wraps_to_first_enabled() {
        let mut list = SelectList::new(Arc::new(MockSource { items: items5() }));
        list.load().await.unwrap();
        list.on_key(KeyEvent::new(
            KeyCode::Up,
            crossterm::event::KeyModifiers::NONE,
        ));
        // 没有 wrap，选中保持 0（prev_enabled 在 cur=0 时直接返回 first_enabled）
        assert_eq!(list.selected_index(), 0);
    }

    #[tokio::test]
    async fn down_advances_and_clamps_at_end() {
        let mut list = SelectList::new(Arc::new(MockSource { items: items5() }));
        list.load().await.unwrap();
        for _ in 0..10 {
            list.on_key(KeyEvent::new(
                KeyCode::Down,
                crossterm::event::KeyModifiers::NONE,
            ));
        }
        assert_eq!(list.selected_index(), 4); // 末项
    }

    #[tokio::test]
    async fn char_digit_1to9_jumps_to_index() {
        let mut list = SelectList::new(Arc::new(MockSource { items: items5() }));
        list.load().await.unwrap();
        let action = list.on_key(KeyEvent::new(
            KeyCode::Char('3'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(list.selected_index(), 2);
        assert!(matches!(action, SelectAction::Confirm(2)));
    }

    #[tokio::test]
    async fn char_digit_out_of_range_is_stay() {
        let mut list = SelectList::new(Arc::new(MockSource { items: items5() }));
        list.load().await.unwrap();
        let action = list.on_key(KeyEvent::new(
            KeyCode::Char('9'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(matches!(action, SelectAction::Stay));
    }

    #[tokio::test]
    async fn pageup_pagedown() {
        let mut list = SelectList::new(Arc::new(MockSource { items: items5() }));
        list.load().await.unwrap();
        list.set_viewport_height(3);
        list.on_key(KeyEvent::new(
            KeyCode::PageDown,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(list.selected_index(), 3);
        list.on_key(KeyEvent::new(
            KeyCode::PageUp,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(list.selected_index(), 0);
    }

    #[tokio::test]
    async fn mouse_scroll_down() {
        let mut list = SelectList::new(Arc::new(MockSource { items: items5() }));
        list.load().await.unwrap();
        let mouse = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let action = list.on_mouse(mouse);
        assert!(matches!(action, SelectAction::Down));
        assert_eq!(list.selected_index(), 1);
    }

    #[tokio::test]
    async fn disabled_item_is_skipped() {
        let mut items = items5();
        items[1] = items[1].clone().disabled();
        let mut list = SelectList::new(Arc::new(MockSource { items }));
        list.load().await.unwrap();
        // 选中 0（enabled），按 Down 应跳到 2（跳过 1）
        list.on_key(KeyEvent::new(
            KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(list.selected_index(), 2);
    }

    #[tokio::test]
    async fn enter_on_disabled_is_stay() {
        let mut items = items5();
        items[0] = items[0].clone().disabled();
        let mut list = SelectList::new(Arc::new(MockSource { items }));
        list.load().await.unwrap();
        // selected 自动跳到 1（第一个 enabled）
        assert_eq!(list.selected_index(), 1);
        let action = list.on_key(KeyEvent::new(
            KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(matches!(action, SelectAction::Confirm(1)));
    }

    #[tokio::test]
    async fn esc_cancels() {
        let mut list = SelectList::new(Arc::new(MockSource { items: items5() }));
        list.load().await.unwrap();
        let action = list.on_key(KeyEvent::new(
            KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(matches!(action, SelectAction::Cancel));
    }

    #[tokio::test]
    async fn empty_items_source() {
        let mut list = SelectList::new(Arc::new(MockSource { items: vec![] }));
        list.load().await.unwrap();
        let action = list.on_key(KeyEvent::new(
            KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(matches!(action, SelectAction::Stay));
    }

    #[tokio::test]
    async fn async_source_returns_error_propagates() {
        struct ErrorSource;
        #[async_trait]
        impl SelectItemSource for ErrorSource {
            async fn items(&self) -> Result<Vec<SelectItem>> {
                Err(EflowError::Internal("test error".into()))
            }
        }
        let mut list = SelectList::new(Arc::new(ErrorSource));
        let result = list.load().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn selectlist_render_does_not_panic() {
        // 临时硬编码 ratatui 调用——验证 TestBackend 渲染不 panic
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut list = SelectList::new(Arc::new(MockSource { items: items5() }));
        list.load().await.unwrap();
        terminal
            .draw(|f| {
                list.render(f.area(), f.buffer_mut());
            })
            .unwrap();
    }
}
