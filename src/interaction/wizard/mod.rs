//! v1.3.1 配置向导子系统
//!
//! 核心零硬编码步骤名：每个 step 1 个 `impl WizardStep`，
//! 通过 `Wizard::new(steps: Vec<Arc<dyn WizardStep>>)` 注册，
//! main.rs 启动时统一注册。`qingbird init` 调 `Wizard::run`。
//!
//! 关键设计决策：
//! - `WizardState` 跨步共享，每步只关心自己读写哪些字段
//! - `next_step()` 返回 `Option<Arc<dyn WizardStep>>`——状态机本身是数据
//! - `skip_protocol_step` 让"preset vs 自定义"分叉

use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::KeyEvent;
use ratatui::backend::CrosstermBackend;

use crate::common::error::Result;
use crate::infrastructure::config::EflowConfig;
use crate::interaction::render::FrameViewModel;
use crate::interaction::render::ScreenViewModel;
use crate::interaction::render::render_engine::{DefaultRenderEngine, RenderEngine};
use crate::interaction::render::view_model::*;
use crate::interaction::tui::execute_draw_commands;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepAction {
    Stay,
    Next,
    Prev,
    Cancel,
}

#[async_trait]
pub trait WizardStep: Send + Sync {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    /// 输出 ViewModel（v1.4 spec D：零 ratatui 硬编码）
    fn view_model(&self, state: &WizardState) -> StepViewModel;
    fn on_key(&self, key: KeyEvent, state: &mut WizardState) -> StepAction;
    fn is_complete(&self, state: &WizardState) -> bool;
    fn next_step(&self) -> Option<Arc<dyn WizardStep>>;
    fn on_enter(&self, _state: &mut WizardState) {}
    fn on_exit(&self, _state: &mut WizardState) {}
}

#[derive(Debug, Clone, Default)]
pub struct WizardState {
    pub config: EflowConfig,
    pub provider_protocol: Option<String>,
    pub provider_base_url: Option<String>,
    pub provider_api_key: Option<String>,
    pub provider_id: Option<String>,
    pub provider_display_name: Option<String>,
    pub default_model: Option<String>,
    pub skip_protocol_step: bool,
    pub fetch_failed: bool,
    pub preset_models: Vec<String>,
}

pub struct Wizard {
    steps: Vec<Arc<dyn WizardStep>>,
}

impl Wizard {
    pub fn new(steps: Vec<Arc<dyn WizardStep>>) -> Self {
        Self { steps }
    }

    /// 同步执行向导（v1.4: 完整 ratatui 主循环 + ViewModel + RenderEngine 路径）
    pub fn run(&self) -> Result<WizardOutcome> {
        let mut state = WizardState::default();
        let mut current = 0;

        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = ratatui::Terminal::new(backend).expect("terminal init");
        let _ = crossterm::terminal::enable_raw_mode();

        let engine = DefaultRenderEngine::new();

        let outcome = loop {
            if current >= self.steps.len() {
                break WizardOutcome::Completed(Box::new(state));
            }
            let step = self.steps[current].clone();
            step.on_enter(&mut state);

            // 单步循环：渲染 + 等待用户输入
            let step_outcome = self.run_single_step(&step, &mut state, &engine, &mut terminal);
            step.on_exit(&mut state);

            match step_outcome {
                StepOutcome::Next => current += 1,
                StepOutcome::Prev => {
                    current = current.saturating_sub(1);
                }
                StepOutcome::Cancel => break WizardOutcome::Cancelled,
            }
        };

        let _ = crossterm::terminal::disable_raw_mode();
        Ok(outcome)
    }

    fn run_single_step(
        &self,
        step: &Arc<dyn WizardStep>,
        state: &mut WizardState,
        engine: &DefaultRenderEngine,
        terminal: &mut ratatui::Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> StepOutcome {
        use crossterm::event::{Event, KeyCode as CtKeyCode, KeyEventKind};
        use std::time::Duration;

        loop {
            // 渲染：step.view_model → engine.render → execute_draw_commands
            let vm = step.view_model(state);
            let frame_vm = FrameViewModel::FullScreen(ScreenViewModel::Wizard(vm));
            let cmds = engine.render(&frame_vm);

            let _ = terminal.draw(|f| {
                execute_draw_commands(&cmds, f.area(), f.buffer_mut());
            });

            // 阻塞等键盘事件（100ms timeout 让外部能打断）
            if crossterm::event::poll(Duration::from_millis(100)).unwrap_or(false)
                && let Ok(Event::Key(key)) = crossterm::event::read()
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    CtKeyCode::Enter => return StepOutcome::Next,
                    CtKeyCode::Esc => return StepOutcome::Cancel,
                    _ => {
                        let action = step.on_key(key, state);
                        match action {
                            StepAction::Next => return StepOutcome::Next,
                            StepAction::Prev => return StepOutcome::Prev,
                            StepAction::Cancel => return StepOutcome::Cancel,
                            StepAction::Stay => continue,
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum WizardOutcome {
    Completed(Box<WizardState>),
    Cancelled,
}

#[derive(Debug, Clone, Copy)]
enum StepOutcome {
    Next,
    Prev,
    Cancel,
}

pub mod builtin;
