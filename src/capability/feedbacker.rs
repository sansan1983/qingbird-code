use std::sync::Arc;
use chrono::Utc;

use crate::common::error::Result;
use crate::common::types::*;
use crate::infrastructure::llm::{ChatRequest, LlmRouter, Message};
use super::blackboard::Blackboard;
use rust_i18n::t;

/// Feedbacker — 质量评估 + 反馈回路
pub struct Feedbacker {
    llm: Arc<tokio::sync::Mutex<LlmRouter>>,
}

impl Feedbacker {
    pub fn new(llm: Arc<tokio::sync::Mutex<LlmRouter>>) -> Self {
        Self { llm }
    }

    /// 评估执行结果并返回判决
    pub async fn evaluate(&self, blackboard: Blackboard) -> Result<(Blackboard, QualityVerdict)> {
        // 规则 1: 无操作记录 → Pass（不调 LLM）
        if blackboard.action_log.is_empty() {
            let record = FeedbackRecord {
                timestamp: Utc::now(),
                verdict: QualityVerdict::Pass {
                    summary: t!("status_feedback_no_actions_detail").to_string(),
                },
                retry_count: blackboard.retry_count,
            };
            return Ok((
                blackboard.with_feedback(record),
                QualityVerdict::Pass {
                    summary: t!("status_feedback_no_actions").to_string(),
                },
            ));
        }

        // 提取所需字段（避免后续 blackboard 借用冲突）
        let all_succeeded = blackboard.action_log.iter().all(|a| a.success);
        let last_action = blackboard
            .action_log
            .last()
            .cloned()
            .expect("checked is_empty above");
        let risk = blackboard.risk_level;
        let count = blackboard.action_log.len();
        let retry_count = blackboard.retry_count;

        // 规则 2: 全部成功 + 风险 ≤ L1 → 快速 Pass（不调 LLM）
        if all_succeeded && risk <= RiskLevel::L1 {
            let summary = t!("status_feedback_all_passed", count = count).to_string();
            let record = FeedbackRecord {
                timestamp: Utc::now(),
                verdict: QualityVerdict::Pass {
                    summary: summary.clone(),
                },
                retry_count,
            };
            return Ok((
                blackboard.with_feedback(record),
                QualityVerdict::Pass { summary },
            ));
        }

        // 复杂情况：调 LLM 评估
        let verdict = self.llm_evaluate(&blackboard, &last_action, retry_count, risk).await?;
        let record = FeedbackRecord {
            timestamp: Utc::now(),
            verdict: verdict.clone(),
            retry_count,
        };
        Ok((blackboard.with_feedback(record), verdict))
    }

    /// 调用 LLM 进行质量评估
    async fn llm_evaluate(
        &self,
        blackboard: &Blackboard,
        last_action: &ActionRecord,
        retry_count: u8,
        risk: RiskLevel,
    ) -> Result<QualityVerdict> {
        let mut llm = self.llm.lock().await;

        let operation_summary: String = blackboard
            .action_log
            .iter()
            .map(|a| {
                let status = if a.success { "✓" } else { "✗" };
                let summary: String = a.summary.chars().take(80).collect();
                format!("{} {}: {}", status, a.tool, summary)
            })
            .collect::<Vec<_>>()
            .join("\n");

        // 用 char 切片避免多字节 UTF-8 边界问题
        let desc: String = blackboard
            .task
            .description
            .chars()
            .take(200)
            .collect();

        let messages = vec![
            Message::system(
                "你是一个质量评估专家。判断任务执行结果是否达标。\n\
                 回复格式:\n\
                 - PASS: <摘要>  (如果结果达标)\n\
                 - REWORK: <原因> | <建议>  (如果需要重做)\n\
                 - ESCALATE: <原因>  (如果需要升级)",
            ),
            Message::user(format!(
                "任务: {}\n风险等级: {:?}\n重试次数: {}\n\n执行记录:\n{}\n\n最后一步工具: {}\n最后一步状态: {}\n\n请评估。",
                desc,
                risk,
                retry_count,
                operation_summary,
                last_action.tool,
                if last_action.success { "成功" } else { "失败" },
            )),
        ];

        let request = ChatRequest::new("", messages);
        let response = llm.chat(ModelTier::Medium, request).await?;

        // 解析 LLM 输出
        let content = response.content.trim();
        if content.starts_with("PASS:") || content.starts_with("PASS：") {
            Ok(QualityVerdict::Pass {
                summary: content[5..].trim().to_string(),
            })
        } else if content.starts_with("REWORK:") || content.starts_with("REWORK：") {
            let body = &content[7..].trim();
            let parts: Vec<&str> = body.splitn(2, '|').collect();
            let reason = parts
                .first()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| t!("err_feedback_unknown_reason").to_string());
            let suggestion = parts
                .get(1)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| t!("err_feedback_retry_suggestion").to_string());
            Ok(QualityVerdict::Rework { reason, suggestion })
        } else if content.starts_with("ESCALATE:") || content.starts_with("ESCALATE：") {
            Ok(QualityVerdict::Escalate {
                reason: content[9..].trim().to_string(),
                new_risk: RiskLevel::L3,
            })
        } else {
            // 无法解析 → 默认 Pass
            Ok(QualityVerdict::Pass {
                summary: content.chars().take(100).collect(),
            })
        }
    }
}
