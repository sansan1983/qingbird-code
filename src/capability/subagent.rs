use uuid::Uuid;

use super::blackboard::Blackboard;
use super::decisioner::Decisioner;
use super::executor::Executor;
use super::feedbacker::Feedbacker;
use crate::common::error::Result;
use crate::common::types::{Capability, PermissionSet, QualityVerdict, RiskLevel, Role};
use rust_i18n::t;

/// Subagent — 实际执行任务的工人（v1.0 单 Subagent）
pub struct Subagent {
    pub id: Uuid,
    pub name: String,
    pub role: Role,
    pub capabilities: Vec<Capability>,
    pub permission: PermissionSet,
    /// v1.2 E5: idle cleanup 用（pool.rs cleanup_idle 扫超时）
    pub created_at: std::time::SystemTime,
}

impl Subagent {
    #[must_use]
    pub fn new(name: String, role: Role, capabilities: Vec<Capability>) -> Self {
        let mut permission = PermissionSet::default();
        // 根据 capabilities 推导权限边界（设计 §9.1 + §13.1）
        if capabilities.contains(&Capability::ExecuteCommand) {
            permission.allowed_commands.push("ls".into());
            permission.allowed_commands.push("cat".into());
        }
        if capabilities.contains(&Capability::WriteFile) {
            permission.max_file_size_bytes = 10 * 1024 * 1024;
        }
        if !capabilities.contains(&Capability::WebFetch) {
            permission.network_enabled = false;
        }
        Self {
            id: Uuid::new_v4(),
            name,
            role,
            capabilities,
            permission,
            created_at: std::time::SystemTime::now(),
        }
    }

    /// 执行单个步骤的完整管线段：D → E → F（含反馈回路，最多 3 次重试）
    pub async fn execute_step(
        &self,
        blackboard: Blackboard,
        decisioner: &Decisioner,
        executor: &Executor,
        feedbacker: &Feedbacker,
    ) -> Result<Blackboard> {
        let mut bb = blackboard;
        let max_retries: u8 = 3;

        loop {
            // Decisioner: 评估 + 规划
            bb = decisioner.decide(&bb).await?;

            // Executor: 执行
            bb = executor.execute(bb).await?;

            // Feedbacker: 评估 + 判决
            let (new_bb, verdict) = feedbacker.evaluate(bb).await?;
            bb = new_bb;

            match verdict {
                QualityVerdict::Pass { .. } => {
                    // 步骤完成
                    return Ok(bb);
                }
                QualityVerdict::Rework {
                    reason: _,
                    suggestion,
                } => {
                    if bb.retry_count >= max_retries {
                        // 超过最大重试，强制升级
                        tracing::warn!("Step exceeded max retries ({}), escalating", max_retries);
                        bb.risk_level = RiskLevel::L3;
                        return Ok(bb);
                    }
                    // 带着建议重试
                    tracing::info!(
                        "Rework needed: {}. Retry {}/{}",
                        suggestion,
                        bb.retry_count + 1,
                        max_retries
                    );
                    bb = bb.increment_retry();
                    // 在当前步骤的 action 上追加修正指令
                    if let Some(ref mut step) = bb.current_step {
                        step.action = t!(
                            "status_subagent_rework_action",
                            action = step.action.clone(),
                            suggestion = suggestion
                        )
                        .to_string();
                    }
                    continue;
                }
                QualityVerdict::Escalate { reason, new_risk } => {
                    tracing::warn!("Escalated to {:?}: {}", new_risk, reason);
                    bb.risk_level = new_risk;
                    return Ok(bb);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_assigns_id_and_fields() {
        let s = Subagent::new(
            "alpha".into(),
            Role::CodeAssistant,
            vec![Capability::ReadFile, Capability::SearchCode],
        );
        assert_eq!(s.name, "alpha");
        assert!(matches!(s.role, Role::CodeAssistant));
        assert_eq!(s.capabilities.len(), 2);
    }

    #[test]
    fn ids_are_unique_across_instances() {
        let a = Subagent::new("a".into(), Role::Generalist, vec![]);
        let b = Subagent::new("b".into(), Role::Generalist, vec![]);
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn default_permission_is_restrictive() {
        let s = Subagent::new("x".into(), Role::Generalist, vec![]);
        assert!(s.permission.allowed_paths.is_empty());
        assert!(s.permission.allowed_commands.is_empty());
        assert!(!s.permission.network_enabled);
    }

    #[test]
    fn execute_command_capability_unlocks_command_permission() {
        let s = Subagent::new(
            "x".into(),
            Role::Generalist,
            vec![Capability::ExecuteCommand],
        );
        assert!(!s.permission.allowed_commands.is_empty());
    }
}
