// TODO(v1.4 spec D): WizardStep::render() / SelectList::render() / TuiBackend
// 渲染部分直接调 ratatui API，违反"零硬编码"原则。
// v1.4 spec D 重构为 RenderEngine trait + DrawCommand enum。
// 见 specs/2026-06-17-eflow-v1.3-b1-wizard-slash-design.md §12 已知偏差。

//! v1.3.1 配置向导子系统
//!
//! 核心零硬编码步骤名：每个 step 1 个 `impl WizardStep`，
//! 通过 `Wizard::new(steps: Vec<Arc<dyn WizardStep>>)` 注册，
//! main.rs 启动时统一注册。`eflow init` 调 `Wizard::run`。
//!
//! 关键设计决策：
//! - `WizardState` 跨步共享，每步只关心自己读写哪些字段
//! - `next_step()` 返回 `Option<Arc<dyn WizardStep>>`——状态机本身是数据
//! - `skip_protocol_step` 让"preset vs 自定义"分叉

use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::prelude::Buffer;

use crate::common::error::Result;
use crate::infrastructure::config::EflowConfig;
use crate::infrastructure::llm::types::ProtocolKind;

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
    /// 渲染（**临时硬编码**——v1.4 spec D 重构）
    fn render(&self, area: Rect, buf: &mut Buffer, state: &WizardState);
    fn on_key(&self, key: KeyEvent, state: &mut WizardState) -> StepAction;
    fn is_complete(&self, state: &WizardState) -> bool;
    fn next_step(&self) -> Option<Arc<dyn WizardStep>>;
    fn on_enter(&self, _state: &mut WizardState) {}
    fn on_exit(&self, _state: &mut WizardState) {}
}

#[derive(Debug, Clone, Default)]
pub struct WizardState {
    pub config: EflowConfig,
    pub provider_protocol: Option<ProtocolKind>,
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

    /// 同步执行向导（spec B1 阶段：纯键盘交互 + ratatui 渲染）
    ///
    /// **临时硬编码**——v1.4 spec D 重构
    pub fn run(&self) -> Result<WizardOutcome> {
        let mut state = WizardState::default();
        let mut current = 0;

        loop {
            if current >= self.steps.len() {
                // 向导完成
                return Ok(WizardOutcome::Completed(Box::new(state)));
            }
            let step = self.steps[current].clone();
            step.on_enter(&mut state);

            // 单步循环：渲染 + 等待用户输入
            // v1.3.1 阶段：每个 step 一次性 on_key 决定 Next/Cancel
            // v1.4 spec D 重构：完整 ratatui 主循环
            let outcome = self.run_single_step(&step, &mut state);
            step.on_exit(&mut state);
            match outcome {
                StepOutcome::Next => current += 1,
                StepOutcome::Prev => {
                    current = current.saturating_sub(1);
                }
                StepOutcome::Cancel => return Ok(WizardOutcome::Cancelled),
            }
        }
    }

    fn run_single_step(&self, _step: &Arc<dyn WizardStep>, state: &mut WizardState) -> StepOutcome {
        // v1.3.1 简化实现：每个 step 通过 stdin 读一行 + parse
        // 真实实现留待 spec D（v1.4）——spec B1 文档 §12 已知偏差
        use std::io::BufRead;
        let stdin = std::io::stdin();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            return StepOutcome::Cancel;
        }
        let line = line.trim();
        match line {
            "" => StepOutcome::Next,
            "esc" | "Esc" | "ESC" => StepOutcome::Cancel,
            "prev" | "Prev" | "PREV" | "back" => StepOutcome::Prev,
            _ => {
                // 把输入存到 state（具体 step 实现自己 parse）
                // 简化：直接把 line 写到 state.config.core.language（各 step 覆盖）
                // 实际实施时各 step 重写这个方法
                let _ = state;
                StepOutcome::Next
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
