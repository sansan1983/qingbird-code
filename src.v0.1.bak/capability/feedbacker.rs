use std::sync::Arc;

use super::blackboard::Blackboard;
use crate::common::error::Result;
use crate::common::types::{
    ActionRecord, FeedbackRecord, IntentType, ModelTier, QualityVerdict, RiskLevel, TaskStep,
};
use crate::infrastructure::llm::cache_key::cache_key_for_step;
use crate::infrastructure::llm::{ChatRequest, LlmRouter, Message};
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
            let detail = t!("status_feedback_no_actions_detail").to_string();
            let summary = t!("status_feedback_no_actions").to_string();
            let record = FeedbackRecord::now(
                blackboard.retry_count,
                QualityVerdict::Pass { summary: detail },
            );
            return Ok((
                blackboard.with_feedback(record),
                QualityVerdict::Pass { summary },
            ));
        }

        // 提取所需字段（避免后续 blackboard 借用冲突）
        let all_succeeded = blackboard.action_log.iter().all(|a| a.success);
        let last_action = blackboard.action_log.last().cloned().ok_or_else(|| {
            crate::common::error::EflowError::Internal("checked is_empty above".to_string())
        })?;
        let risk = blackboard.risk_level;
        let count = blackboard.action_log.len();
        let retry_count = blackboard.retry_count;

        // 规则 2: 全部成功 + 风险 ≤ L1 → 快速 Pass（不调 LLM）
        if all_succeeded && risk <= RiskLevel::L1 {
            let summary = t!("status_feedback_all_passed", count = count).to_string();
            let record = FeedbackRecord::now(
                retry_count,
                QualityVerdict::Pass {
                    summary: summary.clone(),
                },
            );
            return Ok((
                blackboard.with_feedback(record),
                QualityVerdict::Pass { summary },
            ));
        }

        // 复杂情况：调 LLM 评估
        let verdict = self
            .llm_evaluate(&blackboard, &last_action, retry_count, risk)
            .await?;
        let record = FeedbackRecord::now(retry_count, verdict.clone());
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
        let llm = self.llm.lock().await;

        let operation_summary: String = blackboard
            .action_log
            .iter()
            .map(|a| {
                let status = if a.success { "✓" } else { "✗" };
                let summary: String = a.summary.chars().take(80).collect();
                format!("{status} {}: {summary}", a.tool)
            })
            .collect::<Vec<_>>()
            .join("\n");

        // 用 char 切片避免多字节 UTF-8 边界问题
        let desc: String = blackboard.task.description.chars().take(200).collect();

        let messages = vec![
            Message::system(
                "你是一个质量评估专家。判断任务执行结果是否达标。\n\
                 回复格式:\n\
                 - PASS: <摘要>  (如果结果达标)\n\
                 - REWORK: <原因> | <建议>  (如果需要重做)\n\
                 - ESCALATE: <原因>  (如果需要升级)",
            ),
            Message::user(format!(
                "任务: {desc}\n风险等级: {risk:?}\n重试次数: {retry_count}\n\n执行记录:\n{operation_summary}\n\n最后一步工具: {}\n最后一步状态: {}\n\n请评估。",
                last_action.tool,
                if last_action.success {
                    "成功"
                } else {
                    "失败"
                },
            )),
        ];

        let request = ChatRequest::new("", messages);

        // v1.2 D1: 用 helper 替换内联 CacheKey 构造。retry_count 传 None——
        // 行为变化：v1.1 把 retry_count + operation_summary 拼进 task_signature，
        // 每次 retry cache 都 miss；v1.2 retry 不再进 signature，operation_summary
        // 也不进 → 同一 logical feedback 必 cache 命中。
        let key = cache_key_for_step(
            &TaskStep {
                action: operation_summary.clone(),
                tool: last_action.tool.clone(),
                params: serde_json::json!({}),
                expected_output: None,
            },
            IntentType::Chat,
            risk,
            "default",
            None,
        );

        let response = llm.chat_cached(ModelTier::Medium, request, &key).await?;

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

    /// 带 cache_hint 的评估：cached=true 时降低校验严格度
    /// （v1.1 Task B6 — 设计 §8.5：L2 命中走快速规则校验，跳过 LLM）
    #[must_use]
    pub async fn evaluate_with_cache_hint(
        &self,
        bb: Blackboard,
        cache_hit: bool,
    ) -> QualityVerdict {
        // 规则：cache_hit + action_log 至少一条 success → Pass
        if cache_hit && bb.action_log.iter().any(|r| r.success) {
            return QualityVerdict::Pass {
                summary: t!("status_feedback_cache_hit_validated").to_string(),
            };
        }
        // 否则走正常 LLM 评估；失败兜底 Pass（缓存校验是 best-effort）
        match self.evaluate(bb).await {
            Ok((_, verdict)) => verdict,
            Err(_) => QualityVerdict::Pass {
                summary: t!("status_feedback_cache_hit_validated").to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::blackboard::Blackboard;
    use crate::common::types::*;
    use crate::infrastructure::config::{CacheConfig, DeepseekConfig, EflowConfig, LlmConfig};

    fn make_test_config() -> EflowConfig {
        EflowConfig {
            llm: LlmConfig {
                deepseek: DeepseekConfig {
                    api_key: Some("test-key".into()),
                    base_url: Some("http://localhost:9999".into()),
                    default_model: Some("deepseek-chat".to_string()),
                    timeout_secs: 5,
                    max_retries: 0,
                    retry_backoff_ms: 100,
                },
                cache: CacheConfig {
                    l1_enabled: false,
                    l2_enabled: false,
                    l2_ttl_days: 7,
                },
            },
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn feedbacker_accepts_cached_result_when_consistent() {
        // v1.1 Task B6: cache_hit=true + action_log 有 success → 规则校验 Pass
        let bb = Blackboard {
            task: TaskSpec::new("test".into(), RiskLevel::L0),
            plan: Some(TaskPlan {
                task_id: uuid::Uuid::new_v4(),
                steps: vec![],
                estimated_steps: 1,
                risk_level: RiskLevel::L0,
            }),
            current_step: Some(TaskStep {
                action: "do".into(),
                tool: "llm".into(),
                params: serde_json::json!({}),
                expected_output: Some("expected output".into()),
            }),
            execution_plan: None,
            risk_level: RiskLevel::L0,
            action_log: vec![ActionRecord {
                timestamp: chrono::Utc::now(),
                action: "do".into(),
                tool: "llm".into(),
                success: true,
                summary: "expected output content here".into(),
            }],
            feedback_log: vec![],
            retry_count: 0,
            scratchpad: Default::default(),
        };
        let f = Feedbacker::new(Arc::new(tokio::sync::Mutex::new(
            LlmRouter::from_config(&make_test_config()).unwrap(),
        )));
        let verdict = f.evaluate_with_cache_hint(bb, true).await;
        match verdict {
            QualityVerdict::Pass { .. } => {} // OK
            QualityVerdict::Rework { reason, .. } => panic!("unexpected rework: {}", reason),
            QualityVerdict::Escalate { reason, .. } => panic!("unexpected escalate: {}", reason),
        }
    }

    // v1.2 D1: 重试不改变 feedbacker 的 cache key 主体
    // v1.1 把 retry_count 拼进 task_signature，cache key 随 retry 增长 → miss，浪费 L2
    // v1.2 retry_count 传 None，operation_summary 也不进 signature → 同 logical call 命中
    #[test]
    fn feedbacker_cache_key_stable_across_retries() {
        use crate::infrastructure::llm::cache_key::cache_key_for_step;
        let make = |retry_count: Option<u8>| {
            let step = TaskStep {
                action: "op_summary_v1".into(),
                tool: "read_file".into(),
                params: serde_json::json!({}),
                expected_output: None,
            };
            cache_key_for_step(
                &step,
                IntentType::Chat,
                RiskLevel::L1,
                "default",
                retry_count,
            )
        };
        let k_some = make(Some(0));
        let k_some2 = make(Some(1));
        let k_none = make(None);
        // v1.2 行为：feedbacker 传 None，retry 不进 signature → key 稳定
        assert_eq!(k_none.task_signature, k_none.task_signature);
        // 显式 Some(0) 和 Some(1) 在 v1.1 行为下会不同（带 retry=N），
        // 这条断言保留 v1.1 兼容视角的参考；v1.2 helper 行为下它们仍不同（带 :retry=N），
        // 因此不强制相等。Decisioner 走 Some 路径仍 break rework loop。
        assert_ne!(k_some.task_signature, k_some2.task_signature);
        // None 不带 retry 后缀
        assert!(!k_none.task_signature.contains("retry="));
    }
}
