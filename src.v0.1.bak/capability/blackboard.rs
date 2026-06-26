use std::collections::HashMap;

use crate::common::types::{
    ActionRecord, ExecutionPlan, FeedbackRecord, QualityVerdict, RiskLevel, TaskPlan, TaskSpec,
    TaskStep,
};
use rust_i18n::t;

/// 管道内流转的共享上下文（值类型，不可变更新）
#[derive(Debug, Clone)]
pub struct Blackboard {
    pub task: TaskSpec,
    pub plan: Option<TaskPlan>,
    pub current_step: Option<TaskStep>,
    pub execution_plan: Option<ExecutionPlan>,
    pub risk_level: RiskLevel,
    pub action_log: Vec<ActionRecord>,
    pub feedback_log: Vec<FeedbackRecord>,
    pub retry_count: u8,
    pub scratchpad: HashMap<String, serde_json::Value>,
}

impl Blackboard {
    #[must_use]
    pub fn new(task: TaskSpec) -> Self {
        let risk = task.risk_level;
        Self {
            task,
            plan: None,
            current_step: None,
            execution_plan: None,
            risk_level: risk,
            action_log: vec![],
            feedback_log: vec![],
            retry_count: 0,
            scratchpad: HashMap::new(),
        }
    }

    /// 不可变更新 — 返回新版本
    #[must_use]
    pub fn with_plan(mut self, plan: TaskPlan) -> Self {
        self.risk_level = plan.risk_level;
        self.plan = Some(plan);
        self
    }

    #[must_use]
    pub fn with_step(mut self, step: TaskStep) -> Self {
        self.current_step = Some(step);
        self
    }

    #[must_use]
    pub fn with_execution_plan(mut self, plan: ExecutionPlan) -> Self {
        self.risk_level = plan.risk_level;
        self.execution_plan = Some(plan);
        self
    }

    #[must_use]
    pub fn with_action(mut self, record: ActionRecord) -> Self {
        self.action_log.push(record);
        self
    }

    #[must_use]
    pub fn with_feedback(mut self, record: FeedbackRecord) -> Self {
        self.feedback_log.push(record);
        self
    }

    #[must_use]
    pub fn increment_retry(mut self) -> Self {
        self.retry_count += 1;
        self
    }

    /// 生成摘要（用于记忆持久化）
    #[must_use]
    pub fn summarize(&self) -> String {
        let total = self.feedback_log.len();
        let passed = self
            .feedback_log
            .iter()
            .filter(|f| matches!(f.verdict, QualityVerdict::Pass { .. }))
            .count();
        // 用 char 切片避免在多字节 UTF-8 边界切到一半
        let desc: String = self.task.description.chars().take(80).collect();
        t!(
            "status_blackboard_summary",
            desc = desc,
            passed = passed,
            total = total,
            retries = self.retry_count
        )
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::types::{ModelTier, PlannedStep};
    use chrono::Utc;
    use uuid::Uuid;

    fn make_task(desc: &str, risk: RiskLevel) -> TaskSpec {
        TaskSpec::new(desc.into(), risk)
    }

    #[test]
    fn new_uses_task_risk_level() {
        let bb = Blackboard::new(make_task("t", RiskLevel::L2));
        assert_eq!(bb.risk_level, RiskLevel::L2);
        assert!(bb.plan.is_none());
        assert!(bb.action_log.is_empty());
        assert_eq!(bb.retry_count, 0);
    }

    #[test]
    fn with_methods_return_new_versions() {
        let bb0 = Blackboard::new(make_task("t", RiskLevel::L0));
        let plan = TaskPlan {
            task_id: Uuid::new_v4(),
            steps: vec![],
            estimated_steps: 1,
            risk_level: RiskLevel::L1,
        };
        let bb1 = bb0.clone().with_plan(plan.clone());
        assert!(bb1.plan.is_some());
        assert!(bb0.plan.is_none()); // 原版本不变
    }

    #[test]
    fn with_plan_overrides_risk() {
        let bb = Blackboard::new(make_task("t", RiskLevel::L0)).with_plan(TaskPlan {
            task_id: Uuid::new_v4(),
            steps: vec![],
            estimated_steps: 1,
            risk_level: RiskLevel::L3,
        });
        assert_eq!(bb.risk_level, RiskLevel::L3);
    }

    #[test]
    fn with_execution_plan_overrides_risk() {
        let bb =
            Blackboard::new(make_task("t", RiskLevel::L0)).with_execution_plan(ExecutionPlan {
                step: PlannedStep {
                    order: 0,
                    action: "a".into(),
                    tool: "read_file".into(),
                    params: serde_json::json!({}),
                    depends_on: None,
                },
                model_tier: ModelTier::Strong,
                risk_level: RiskLevel::L2,
                sub_steps: vec![],
            });
        assert_eq!(bb.risk_level, RiskLevel::L2);
    }

    #[test]
    fn action_log_appends_in_order() {
        let bb = Blackboard::new(make_task("t", RiskLevel::L0))
            .with_action(ActionRecord {
                timestamp: Utc::now(),
                action: "a1".into(),
                tool: "t".into(),
                success: true,
                summary: "s1".into(),
            })
            .with_action(ActionRecord {
                timestamp: Utc::now(),
                action: "a2".into(),
                tool: "t".into(),
                success: false,
                summary: "s2".into(),
            });
        assert_eq!(bb.action_log.len(), 2);
        assert_eq!(bb.action_log[0].action, "a1");
        assert!(!bb.action_log[1].success);
    }

    #[test]
    fn increment_retry_adds_one() {
        let bb = Blackboard::new(make_task("t", RiskLevel::L0))
            .increment_retry()
            .increment_retry();
        assert_eq!(bb.retry_count, 2);
    }

    #[test]
    fn summarize_counts_passed_vs_total() {
        let bb = Blackboard::new(make_task("t", RiskLevel::L0))
            .with_feedback(FeedbackRecord {
                timestamp: Utc::now(),
                verdict: QualityVerdict::Pass {
                    summary: "ok".into(),
                },
                retry_count: 0,
            })
            .with_feedback(FeedbackRecord {
                timestamp: Utc::now(),
                verdict: QualityVerdict::Rework {
                    reason: "r".into(),
                    suggestion: "s".into(),
                },
                retry_count: 0,
            });
        let s = bb.summarize();
        // 1 passed / 2 total
        assert!(s.contains("1/2"), "got: {}", s);
    }

    #[test]
    fn summarize_handles_multibyte_description() {
        // 80 个中文字符（240 字节）— 不能在 byte 边界切
        let desc: String = "中".repeat(80);
        let bb = Blackboard::new(make_task(&desc, RiskLevel::L0));
        // 不应 panic；切到 80 个字符
        let s = bb.summarize();
        assert!(s.contains(&"中".repeat(10)) || s.contains("..."));
    }

    #[test]
    #[serial_test::serial]
    fn summarize_uses_zh_locale() {
        crate::infrastructure::locale::init(Some("zh-CN"));
        let bb = Blackboard::new(make_task("测试", RiskLevel::L0)).with_feedback(FeedbackRecord {
            timestamp: Utc::now(),
            verdict: QualityVerdict::Pass {
                summary: "ok".into(),
            },
            retry_count: 0,
        });
        let s = bb.summarize();
        assert!(s.contains("任务") || s.contains("步骤"));
    }

    #[test]
    #[serial_test::serial]
    fn summarize_uses_en_locale() {
        crate::infrastructure::locale::init(Some("en-US"));
        let bb = Blackboard::new(make_task("task", RiskLevel::L0)).with_feedback(FeedbackRecord {
            timestamp: Utc::now(),
            verdict: QualityVerdict::Pass {
                summary: "ok".into(),
            },
            retry_count: 0,
        });
        let s = bb.summarize();
        assert!(s.contains("Task"), "got: {}", s);
        // 还原为 zh-CN，避免污染后续测试
        crate::infrastructure::locale::init(Some("zh-CN"));
    }
}
