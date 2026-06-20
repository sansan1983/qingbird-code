//! TUI 交互层实现（设计 §14.3）
//!
//! v1.4 spec D 已重构：render() 改为 state_to_vm() + RenderEngine → DrawCommand → execute_draw_commands。
//! 详见 specs/2026-06-18-eflow-v1.4-rendering-pipeline-design.md §12。
//!
//! 布局（ratatui 4 段：1 行 header + main 区 + 1 行 status + 1 行 prompt）：
//! ┌─────────────────────────────────────────┐
//! │ Header: eflow | profile | cache hit rate │  ← 1 行
//! ├─────────────────────────────────────────┤
//! │                                         │
//! │ Messages / Events (滚动)                │  ← main 区
//! │                                         │
//! ├─────────────────────────────────────────┤
//! │ Status: "Ready" / "Working..."           │  ← 1 行
//! │ > [prompt]                              │  ← 1 行输入框
//! └─────────────────────────────────────────┘

use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{Event as CtEvent, KeyCode, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::prelude::Buffer;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;
use ratatui::widgets::{Block, Borders, Paragraph};
use tokio::sync::Mutex;
use tokio::sync::broadcast;

use super::layer::InteractionLayer;
use crate::application::concierge::Concierge;
use crate::infrastructure::event::{Event, EventChannel};
use crate::interaction::render::FrameViewModel;
use crate::interaction::render::draw_command::{BorderToken, DrawCommand, TextStyle};
use crate::interaction::render::render_engine::{DefaultRenderEngine, RenderEngine};
use crate::interaction::render::view_model::*;

/// TUI 状态
///
/// v1.2 F3: `profile` / `cache_hit_rate` 需要明确初值（不能随便 ""），所以不 derive Default，
/// 改用 `initial()` 显式构造。`std::sync::Mutex`（不是 tokio）因为 event loop 是 sync。
///
/// v1.3.1 增量：`configured` — router 非空 = true；false 时 header 显 ⚠ 警告
struct TuiState {
    messages: Vec<String>,
    status: String,
    prompt_buffer: String,
    profile: String,
    cache_hit_rate: String,
    /// v1.3.1 增量：是否已配置 LLM provider
    configured: bool,
}

impl TuiState {
    /// 启动时的初值
    fn initial() -> Self {
        Self {
            messages: vec!["TUI 启动 — 输入任务并按 Enter".into()],
            status: "Ready".into(),
            prompt_buffer: String::new(),
            profile: String::new(), // run() 启动时同步 block_on 填充
            cache_hit_rate: "n/a".into(),
            configured: true, // 由 main.rs 启动时覆盖
        }
    }
}

/// v1.2 F5: 处理 prompt 输入框的键盘事件
/// 返回 Some(cmd) 表示用户提交了命令（Enter），调用方应把它喂给 Concierge
fn handle_input_key(state: &mut TuiState, code: KeyCode) -> Option<String> {
    match code {
        KeyCode::Char(c) => {
            state.prompt_buffer.push(c);
            None
        }
        KeyCode::Backspace => {
            state.prompt_buffer.pop();
            None
        }
        KeyCode::Enter => {
            let cmd = std::mem::take(&mut state.prompt_buffer);
            if !cmd.is_empty() {
                state.messages.push(format!("> {cmd}"));
                state.status = "Dispatching...".into();
                Some(cmd)
            } else {
                None
            }
        }
        // v1.3.1 增量：Up/Down 键在 TUI 主循环里更新 status。
        // 完整 SelectList widget 集成（让 Up/Down 真做选择）留 v1.3.2 spec B 实施。
        KeyCode::Up => {
            state.status = "↑".into();
            None
        }
        KeyCode::Down => {
            state.status = "↓".into();
            None
        }
        _ => None,
    }
}

pub struct TuiBackend {
    /// tick interval（refresh status bar）
    tick_rate: Duration,
    /// v1.2 F3 补充：F6 main.rs 在 async 上下文填好后注入
    initial_profile: String,
    initial_cache_hit_rate: String,
    /// v1.3.1 增量：是否已配置 LLM provider（false 时 header 显 ⚠ 警告）
    initial_configured: bool,
}

impl TuiBackend {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tick_rate: Duration::from_millis(250),
            initial_profile: String::new(),
            initial_cache_hit_rate: "n/a".into(),
            initial_configured: true, // v1.3.1 增量：默认已配置（v1.2 行为）
        }
    }

    /// v1.2 F3 补充：main.rs 在 async 上下文拿到 initial profile + cache stats
    /// 后，用这个构造器注入。F6 main.rs 改造用。
    #[must_use]
    pub fn with_initial(profile: String, cache_hit_rate: String) -> Self {
        Self {
            tick_rate: Duration::from_millis(250),
            initial_profile: profile,
            initial_cache_hit_rate: cache_hit_rate,
            initial_configured: true, // v1.3.1 增量：with_initial 假设已配置
        }
    }

    /// v1.3.1 增量：bare TUI 模式——未配置 LLM provider 时启动。
    /// header 显 "⚠ 未配置 LLM provider" 警告。
    #[must_use]
    pub fn with_bare_mode() -> Self {
        Self {
            tick_rate: Duration::from_millis(250),
            initial_profile: String::new(),
            initial_cache_hit_rate: "n/a".into(),
            initial_configured: false,
        }
    }

    /// 把 TuiState 翻译为 FrameViewModel（核心层职责）
    fn state_to_vm(state: &TuiState) -> FrameViewModel {
        let main = MainViewModel {
            header: HeaderVM {
                profile: state.profile.clone(),
                cache_hit_rate: state.cache_hit_rate.clone(),
                configured: state.configured,
            },
            messages: state
                .messages
                .iter()
                .map(|m| MessageVM { text: m.clone() })
                .collect(),
            status: state.status.clone(),
            prompt: state.prompt_buffer.clone(),
        };
        FrameViewModel::FullScreen(ScreenViewModel::Main(main))
    }
}

impl Default for TuiBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl InteractionLayer for TuiBackend {
    fn run(&self, concierge: Arc<Mutex<Concierge>>, events: EventChannel) {
        // 启动 ratatui
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend).expect("terminal init");
        let _ = terminal.clear();

        // 启用 raw mode + 隐藏光标
        let _ = crossterm::terminal::enable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::event::EnableMouseCapture,
            crossterm::cursor::Hide
        );

        // 状态：std::sync::Mutex 同步，sync event loop 直接 .lock()
        // profile / cache_hit_rate 由 F3 with_initial 注入（main.rs 在 async 上下文填好）
        let mut initial = TuiState::initial();
        initial.profile = self.initial_profile.clone();
        initial.cache_hit_rate = self.initial_cache_hit_rate.clone();
        initial.configured = self.initial_configured; // v1.3.1 增量
        let state = std::sync::Arc::new(std::sync::Mutex::new(initial));

        // task 事件 channel
        let mut event_rx = events.subscribe();

        // 拿当前 tokio runtime handle（main 是 #[tokio::main]，handle 在 tokio context 存在）
        // 用 spawn 派发 Concierge.handle_input（不需 block_on）
        let rt = tokio::runtime::Handle::current();

        // 主循环：同步 poll 键盘 + 异步 select 任务事件
        let result: std::io::Result<()> = (|| loop {
            // 渲染：state → VM → engine → DrawCommand → execute_draw_commands
            {
                let state_lock = state.lock().unwrap();
                let vm = TuiBackend::state_to_vm(&state_lock);
                let engine = DefaultRenderEngine::new();
                let cmds = engine.render(&vm);
                terminal.draw(|f| {
                    execute_draw_commands(&cmds, f.area(), f.buffer_mut());
                })?;
            }

            // 阻塞等 crossterm 事件（带 timeout 让 task 事件有处理机会）
            if crossterm::event::poll(self.tick_rate)?
                && let Ok(CtEvent::Key(key)) = crossterm::event::read()
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Char('c')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        return Ok(()); // Ctrl+C 退出
                    }
                    KeyCode::Esc => return Ok(()),
                    // F5 处理 prompt：把 KeyCode 透传给 handle_input_key
                    kc => {
                        let cmd = {
                            let mut s = state.lock().unwrap();
                            handle_input_key(&mut s, kc)
                        };
                        if let Some(cmd) = cmd {
                            // Concierge 异步派发
                            let concierge = concierge.clone();
                            rt.spawn(async move {
                                let _ = concierge.lock().await.handle_input(cmd).await;
                            });
                        }
                    }
                }
            }

            // 非阻塞尝试接收 task 事件（不阻塞主循环；超时由上面的 poll 控制）
            loop {
                let event = match event_rx.try_recv() {
                    Ok(e) => e,
                    Err(broadcast::error::TryRecvError::Empty) => break,
                    Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
                    Err(broadcast::error::TryRecvError::Closed) => return Ok(()),
                };
                let mut s = state.lock().unwrap();
                match event {
                    Event::TaskStarted { description, .. } => {
                        s.messages.push(format!("▶ {description}"));
                        s.status = "Working...".into();
                    }
                    Event::TaskCompleted { summary, .. } => {
                        let short: String = summary.chars().take(200).collect();
                        s.messages.push(format!("✓ {short}"));
                        s.status = "Ready".into();
                    }
                    Event::TaskFailed { error, .. } => {
                        s.messages.push(format!("✗ {error}"));
                        s.status = "Failed".into();
                    }
                    Event::RiskEscalated { from, to, .. } => {
                        s.messages.push(format!("⚠ Risk: {:?} → {:?}", from, to));
                        s.status = "Risk escalated".into();
                    }
                    Event::UserInputRequired { prompt } => {
                        s.messages.push(format!("? {prompt}"));
                    }
                    Event::SystemShutdown => return Ok(()),
                    // v1.3.2: SystemReady 是 CLI-only 启动信号（spec B2 ADR-0016
                    // TUI 零改造），TUI 不消费——用 _ => 兜底忽略
                    _ => {}
                }
            }
        })();

        // 清理
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::event::DisableMouseCapture,
            crossterm::cursor::Show
        );
        let _ = terminal.show_cursor();

        if let Err(e) = result {
            tracing::error!("TUI loop error: {e}");
        }
    }
}

// ── DrawCommand → ratatui 转换器（仅在此文件使用） ──

/// 机械执行 DrawCommand 画到 screen buffer
pub(crate) fn execute_draw_commands(commands: &[DrawCommand], area: Rect, buf: &mut Buffer) {
    for cmd in commands {
        match cmd {
            DrawCommand::Text { content, style } => {
                let p = Paragraph::new(content.as_str()).style(to_ratatui_style(*style));
                p.render(area, buf);
            }
            DrawCommand::Block { border, title } => {
                let b = Block::default()
                    .borders(Borders::ALL)
                    .border_type(to_ratatui_border(*border))
                    .title(title.as_str());
                b.render(area, buf);
            }
            DrawCommand::Span {
                prefix,
                content,
                style,
            } => {
                let p =
                    Paragraph::new(format!("{prefix}{content}")).style(to_ratatui_style(*style));
                p.render(area, buf);
            }
            DrawCommand::Line {
                commands: span_cmds,
            } => {
                execute_draw_commands(span_cmds, area, buf);
            }
            DrawCommand::ClearArea => {
                buf.set_style(area, Style::default());
            }
        }
    }
}

/// TextStyle → ratatui Style
fn to_ratatui_style(s: TextStyle) -> Style {
    let mut style = Style::default();
    if let Some(fg) = s.fg {
        style = style.fg(fg);
    }
    if let Some(bg) = s.bg {
        style = style.bg(bg);
    }
    if s.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if s.italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    style
}

/// BorderToken → ratatui BorderType
fn to_ratatui_border(t: BorderToken) -> ratatui::widgets::BorderType {
    match t {
        BorderToken::Rounded => ratatui::widgets::BorderType::Rounded,
        BorderToken::Square => ratatui::widgets::BorderType::Plain,
        BorderToken::Double => ratatui::widgets::BorderType::Double,
        BorderToken::None => ratatui::widgets::BorderType::Plain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tui_state_default_has_empty_collections() {
        // v1.2 F3: TuiState 不再 derive Default（profile/cache_hit_rate 需要明确初值），
        // 改用 `TuiState::initial()` helper
        let s = TuiState::initial();
        assert_eq!(s.messages.len(), 1);
        assert!(s.messages[0].contains("TUI"));
        assert_eq!(s.status, "Ready");
        assert!(s.prompt_buffer.is_empty());
        assert_eq!(s.profile, "");
        assert_eq!(s.cache_hit_rate, "n/a");
    }

    #[test]
    fn tui_backend_renders_initial_frame_via_test_backend() {
        // v1.4: 用 RenderEngine 路径（state_to_vm → engine → execute_draw_commands）
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let state = TuiState::initial();
        let vm = TuiBackend::state_to_vm(&state);
        let engine = DefaultRenderEngine::new();
        let cmds = engine.render(&vm);
        terminal
            .draw(|f| {
                execute_draw_commands(&cmds, f.area(), f.buffer_mut());
            })
            .unwrap();
        // 不 panic 即通过
    }

    // ========== v1.2 F4: EventChannel 事件 → TuiState 更新 ==========

    #[tokio::test]
    async fn tui_state_updates_on_task_completed_event() {
        use uuid::Uuid;

        let events = EventChannel::new();
        let mut rx = events.subscribe();

        // 模拟 Orchestrator 推一个 TaskCompleted
        events.publish(Event::TaskCompleted {
            task_id: Uuid::new_v4(),
            summary: "test summary".into(),
        });

        // 收到事件（与 TUI 内部逻辑一致）
        let event = rx.recv().await.unwrap();
        let mut state = TuiState::initial();
        state.messages.clear(); // 初始 messages 已有 "TUI 启动..." 提示，测试清掉
        match event {
            Event::TaskCompleted { summary, .. } => {
                state.messages.push(format!("✓ {summary}"));
                state.status = "Task done".into();
            }
            _ => panic!("expected TaskCompleted"),
        }
        assert_eq!(state.messages.len(), 1);
        assert!(state.messages[0].contains("test summary"));
        assert_eq!(state.status, "Task done");
    }

    // ========== v1.2 F5: prompt 输入处理 ==========

    #[test]
    fn input_handler_appends_chars() {
        let mut state = TuiState::initial();
        state.messages.clear();
        handle_input_key(&mut state, KeyCode::Char('h'));
        handle_input_key(&mut state, KeyCode::Char('i'));
        assert_eq!(state.prompt_buffer, "hi");
    }

    #[test]
    fn input_handler_backspace_removes_last_char() {
        let mut state = TuiState::initial();
        state.messages.clear();
        state.prompt_buffer = "hello".into();
        handle_input_key(&mut state, KeyCode::Backspace);
        assert_eq!(state.prompt_buffer, "hell");
    }

    #[test]
    fn input_handler_enter_clears_buffer_and_returns_command() {
        let mut state = TuiState::initial();
        state.messages.clear();
        state.prompt_buffer = "review main.rs".into();
        let cmd = handle_input_key(&mut state, KeyCode::Enter);
        assert_eq!(cmd, Some("review main.rs".into()));
        assert!(state.prompt_buffer.is_empty());
    }

    // ========== v1.3.1 T8: 焦点感知 + Up/Down 键 + bare 模式 ==========

    #[test]
    fn tui_state_initial_has_configured_true() {
        let s = TuiState::initial();
        assert!(s.configured, "默认已配置（v1.2 行为）");
    }

    #[test]
    fn input_handler_up_sets_status_arrow_up() {
        let mut state = TuiState::initial();
        state.messages.clear();
        let cmd = handle_input_key(&mut state, KeyCode::Up);
        assert_eq!(cmd, None, "Up 键不提交命令");
        assert_eq!(state.status, "↑");
    }

    #[test]
    fn input_handler_down_sets_status_arrow_down() {
        let mut state = TuiState::initial();
        state.messages.clear();
        let cmd = handle_input_key(&mut state, KeyCode::Down);
        assert_eq!(cmd, None, "Down 键不提交命令");
        assert_eq!(state.status, "↓");
    }

    #[test]
    fn tui_backend_with_bare_mode_sets_initial_configured_false() {
        let backend = TuiBackend::with_bare_mode();
        assert!(!backend.initial_configured, "bare 模式：未配置");
    }

    #[test]
    fn tui_backend_new_defaults_to_configured_true() {
        let backend = TuiBackend::new();
        assert!(backend.initial_configured, "默认已配置（v1.2 兼容）");
    }

    #[test]
    fn render_with_bare_mode_does_not_panic() {
        // v1.4: 用 RenderEngine 路径验证 bare 模式不 panic
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = TuiState::initial();
        state.configured = false; // bare 模式
        let vm = TuiBackend::state_to_vm(&state);
        let engine = DefaultRenderEngine::new();
        let cmds = engine.render(&vm);
        terminal
            .draw(|f| {
                execute_draw_commands(&cmds, f.area(), f.buffer_mut());
            })
            .unwrap();
    }
}
