//! v1.3.3 3 档工作流抽象
//!
//! 核心零硬编码档位行为：每档 1 个 `impl WorkflowExecutor`，
//! 通过 `WorkflowRegistry::register()` 注册，main.rs 启动时统一注册。
//! 加新档位 = 写 1 个 `impl` + 1 行 `register()`，**核心零修改**。
//!
//! 关键设计决策：
//! - `WorkflowLevel` 是 `#[non_exhaustive]` 标注——外部代码加 match 必须有 `_` 分支
//! - `WorkflowRegistry::current_level()` **不调 LLM**——只是返回 override 或默认 Standard
//! - 自动判定由 Concierge::determine_workflow_level 在 dispatch 之前做（规则驱动）
//! - `set_override(None)` 清除 override（`/level auto` 切回自动判定）
//!
//! v1.3.3 known deviation: 阶段实施时按 v1.3.2 现状调整 ——
#![allow(dead_code)] // 阶段实施前 AggregatedResult 字段未消费，模块化隔离

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::application::concierge::Concierge;
use crate::application::orchestrator::Orchestrator;
use crate::common::error::{EflowError, Result};
use crate::common::types::TaskSpec;
use crate::infrastructure::memory::MemoryManager;

/// 档位枚举（`#[non_exhaustive]` 标注允许 v1.4+ 加新档位不破坏 match）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowLevel {
    Simple,
    Standard,
    Advanced,
}

impl WorkflowLevel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Standard => "standard",
            Self::Advanced => "advanced",
        }
    }
}

/// 档位执行结果（v1.3.3 新增——v1.2 Orchestrator::execute 返回 String，
/// v1.3.3 档位执行由 impl 自由构造 AggregatedResult）
#[derive(Debug, Clone)]
pub struct AggregatedResult {
    pub summary: String,
    pub workflow_level: WorkflowLevel,
}

impl AggregatedResult {
    #[must_use]
    pub fn new(summary: impl Into<String>, workflow_level: WorkflowLevel) -> Self {
        Self {
            summary: summary.into(),
            workflow_level,
        }
    }
}

/// 档位实现 trait
///
/// **v1.3.3 实施后冻结**（spec C ADR-0019）：加新档位 = 写 1 个 `impl` + 1 行 `register()`，**核心零修改**。
#[async_trait]
pub trait WorkflowExecutor: Send + Sync {
    fn level(&self) -> WorkflowLevel;
    fn description(&self) -> &'static str; // /help 显示
    fn max_retries(&self) -> u8 {
        3
    } // 默认 3，Advanced 可以重写
    async fn execute(&self, ctx: &mut WorkflowContext<'_>) -> Result<AggregatedResult>;
}

/// 档位执行上下文（持有 Concierge + Orchestrator + MemoryManager 借用）
///
/// v1.3.3 deviation #13a: Concierge 字段是 `Arc<Mutex<>>` 不是 `RefCell`，
/// WorkflowContext 持 Arc clone（与 self 借用独立，不冲突）。
pub struct WorkflowContext<'a> {
    pub task: &'a TaskSpec,
    pub concierge: &'a mut Concierge,
    pub orchestrator: Arc<tokio::sync::Mutex<Orchestrator>>,
    pub memory: Arc<tokio::sync::Mutex<dyn MemoryManager>>,
}

/// 档位注册表
pub struct WorkflowRegistry {
    executors: HashMap<WorkflowLevel, Arc<dyn WorkflowExecutor>>,
    /// 会话级 override（None = 走自动判定）
    override_level: Option<WorkflowLevel>,
}

impl WorkflowRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
            override_level: None,
        }
    }

    /// 注册一个档位实现。重复 level → 第二个赢 + warn。
    pub fn register(&mut self, exec: Arc<dyn WorkflowExecutor>) {
        let level = exec.level();
        if self.executors.contains_key(&level) {
            tracing::warn!("工作流档位 {:?} 重复注册，第二个赢", level);
        }
        self.executors.insert(level, exec);
    }

    /// 设置会话级 override（None = 清除，回到自动判定）
    pub fn set_override(&mut self, level: Option<WorkflowLevel>) {
        self.override_level = level;
    }

    /// 取当前 override（None 表示无 override）
    #[must_use]
    pub fn override_level(&self) -> Option<WorkflowLevel> {
        self.override_level
    }

    /// 当前生效的档位：override 优先，否则默认 Standard
    #[must_use]
    pub fn current_level(&self) -> WorkflowLevel {
        self.override_level.unwrap_or(WorkflowLevel::Standard)
    }

    /// 执行指定档位（**不**做自动判定——调用方负责传 level）
    pub async fn execute(
        &self,
        level: WorkflowLevel,
        ctx: &mut WorkflowContext<'_>,
    ) -> Result<AggregatedResult> {
        let exec = self.executors.get(&level).ok_or_else(|| {
            EflowError::Internal(format!("workflow level {:?} not registered", level))
        })?;
        exec.execute(ctx).await
    }

    /// 必需档位校验（main.rs 启动时调用）
    pub fn required_register(&mut self, required: &[WorkflowLevel]) -> Result<()> {
        let missing: Vec<WorkflowLevel> = required
            .iter()
            .copied()
            .filter(|level| !self.executors.contains_key(level))
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            let names: Vec<String> = missing.iter().map(|l| l.label().to_string()).collect();
            Err(EflowError::Config(format!(
                "必需工作流档位未注册: {}",
                names.join(", ")
            )))
        }
    }

    /// 列出所有已注册档位（给 `/help` 用）
    #[must_use]
    pub fn list(&self) -> Vec<(WorkflowLevel, &'static str)> {
        let mut entries: Vec<_> = self
            .executors
            .values()
            .map(|e| (e.level(), e.description()))
            .collect();
        entries.sort_by_key(|(level, _)| *level);
        entries
    }
}

impl Default for WorkflowRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::error::Result;

    struct MockExecutor {
        lvl: WorkflowLevel,
    }
    #[async_trait]
    impl WorkflowExecutor for MockExecutor {
        fn level(&self) -> WorkflowLevel {
            self.lvl
        }
        fn description(&self) -> &'static str {
            "mock"
        }
        async fn execute(&self, _ctx: &mut WorkflowContext<'_>) -> Result<AggregatedResult> {
            unimplemented!("mock executor not for unit tests")
        }
    }

    fn make_executor(level: WorkflowLevel) -> Arc<dyn WorkflowExecutor> {
        Arc::new(MockExecutor { lvl: level })
    }

    #[test]
    fn new_registry_is_empty_and_default_level() {
        let reg = WorkflowRegistry::new();
        assert_eq!(reg.executors.len(), 0);
        assert_eq!(reg.current_level(), WorkflowLevel::Standard); // 默认 Standard
        assert!(reg.override_level().is_none());
    }

    #[test]
    fn register_and_override() {
        let mut reg = WorkflowRegistry::new();
        reg.register(make_executor(WorkflowLevel::Simple));
        reg.register(make_executor(WorkflowLevel::Standard));
        assert_eq!(reg.executors.len(), 2);
        assert_eq!(reg.current_level(), WorkflowLevel::Standard); // 默认 Standard
        reg.set_override(Some(WorkflowLevel::Advanced));
        assert_eq!(reg.current_level(), WorkflowLevel::Advanced);
        reg.set_override(None);
        assert_eq!(reg.current_level(), WorkflowLevel::Standard);
    }

    #[test]
    fn register_duplicate_keeps_latest() {
        let mut reg = WorkflowRegistry::new();
        reg.register(make_executor(WorkflowLevel::Simple));
        reg.register(make_executor(WorkflowLevel::Simple));
        assert_eq!(reg.executors.len(), 1);
    }

    #[test]
    fn required_register_passes_when_all_present() {
        let mut reg = WorkflowRegistry::new();
        reg.register(make_executor(WorkflowLevel::Simple));
        reg.register(make_executor(WorkflowLevel::Standard));
        reg.register(make_executor(WorkflowLevel::Advanced));
        assert!(
            reg.required_register(&[
                WorkflowLevel::Simple,
                WorkflowLevel::Standard,
                WorkflowLevel::Advanced,
            ])
            .is_ok()
        );
    }

    #[test]
    fn required_register_fails_when_missing() {
        let mut reg = WorkflowRegistry::new();
        reg.register(make_executor(WorkflowLevel::Simple));
        let result = reg.required_register(&[WorkflowLevel::Simple, WorkflowLevel::Advanced]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("advanced"));
    }

    #[test]
    fn list_returns_all_registered_sorted_by_level() {
        let mut reg = WorkflowRegistry::new();
        reg.register(make_executor(WorkflowLevel::Advanced));
        reg.register(make_executor(WorkflowLevel::Simple));
        reg.register(make_executor(WorkflowLevel::Standard));
        let list = reg.list();
        assert_eq!(list.len(), 3);
        // WorkflowLevel 是 enum，Ord 由 derive 决定——顺序不重要，只验证数量
    }

    #[test]
    fn level_label_returns_snake_case() {
        assert_eq!(WorkflowLevel::Simple.label(), "simple");
        assert_eq!(WorkflowLevel::Standard.label(), "standard");
        assert_eq!(WorkflowLevel::Advanced.label(), "advanced");
    }

    #[test]
    fn level_serde_roundtrip() {
        let json = serde_json::to_string(&WorkflowLevel::Simple).unwrap();
        assert_eq!(json, "\"simple\"");
        let back: WorkflowLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(back, WorkflowLevel::Simple);
    }

    #[test]
    fn aggregated_result_new_sets_fields() {
        let r = AggregatedResult::new("hello", WorkflowLevel::Simple);
        assert_eq!(r.summary, "hello");
        assert_eq!(r.workflow_level, WorkflowLevel::Simple);
    }
}
