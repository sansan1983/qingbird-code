//! Cache Key 派生逻辑
//!
//! 设计见 spec §8.2-8.4。Key 由 (intent_type, task_signature, context_profile, model)
//! 四元组派生 64-bit hash。ContextProfile 存分桶而非原始值，保证泛化。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::common::types::{IntentType, TaskStep};
use crate::infrastructure::llm::cache::{CacheKey, ContextProfile};

/// Key 派生一个 64-bit 哈希用于存储
pub fn key_hash(key: &CacheKey) -> u64 {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    h.finish()
}

/// v1.2 D1: 把 capability 三处内联 CacheKey 构造抽到一处。
#[must_use]
pub fn cache_key_for_step(
    step: &TaskStep,
    intent_type: IntentType,
    risk_level: crate::common::types::RiskLevel,
    profile_name: &str,
    retry_count: Option<u8>,
) -> CacheKey {
    let base = format!(
        "{}:{}:{}",
        intent_label(intent_type),
        step.tool,
        step.action
    );
    let task_signature = match retry_count {
        Some(r) => format!("{base}:retry={r}"),
        None => base,
    };
    CacheKey {
        intent_type,
        task_signature,
        context_profile: ContextProfile {
            conversation_depth_bucket: 0,
            file_count_bucket: 0,
            risk_level,
            profile_name: profile_name.to_string(),
        },
        model: String::new(),
    }
}

fn intent_label(it: IntentType) -> &'static str {
    use crate::common::types::IntentType;
    match it {
        IntentType::CodeReview => "code_review",
        IntentType::BugFix => "bug_fix",
        IntentType::DataQuery => "data_query",
        IntentType::FileRead => "file_read",
        IntentType::FileWrite => "file_write",
        IntentType::CommandExecute => "command_execute",
        IntentType::WebFetch => "web_fetch",
        IntentType::Chat => "chat",
        IntentType::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::types::RiskLevel;
    use crate::infrastructure::llm::cache::{CacheKey, ContextProfile};

    fn make_key() -> CacheKey {
        CacheKey {
            intent_type: IntentType::CodeReview,
            task_signature: "sig".into(),
            context_profile: ContextProfile {
                conversation_depth_bucket: 0,
                file_count_bucket: 0,
                risk_level: RiskLevel::L0,
                profile_name: "dev".into(),
            },
            model: "m".into(),
        }
    }

    #[test]
    fn cache_key_hash_is_stable() {
        let k = CacheKey {
            intent_type: IntentType::CodeReview,
            task_signature: "read_file_find_pattern".into(),
            context_profile: ContextProfile {
                conversation_depth_bucket: 1,
                file_count_bucket: 1,
                risk_level: RiskLevel::L0,
                profile_name: "developer".into(),
            },
            model: "claude-sonnet-4-6".into(),
        };
        assert_eq!(key_hash(&k), key_hash(&k));
    }

    #[test]
    fn cache_key_differs_on_model() {
        let mut k1 = make_key();
        k1.model = "claude-sonnet-4-6".into();
        let mut k2 = make_key();
        k2.model = "claude-opus-4-8".into();
        assert_ne!(key_hash(&k1), key_hash(&k2));
    }

    #[test]
    fn cache_key_for_step_uses_action_and_tool_only() {
        let step = TaskStep {
            action: "review code".into(),
            tool: "read_file".into(),
            params: serde_json::json!({"path": "/tmp/foo.rs"}),
            expected_output: None,
        };
        let key = cache_key_for_step(
            &step,
            IntentType::CodeReview,
            RiskLevel::L0,
            "default",
            None,
        );
        assert_eq!(key.task_signature, "code_review:read_file:review code");
        assert!(
            key.model.is_empty(),
            "model 字段由 Router 注入，不在 helper 写死"
        );
    }

    #[test]
    fn cache_key_hash_differs_on_action_change() {
        let k1 = cache_key_for_step(
            &TaskStep {
                action: "read foo.rs".into(),
                tool: "read_file".into(),
                params: serde_json::json!({}),
                expected_output: None,
            },
            IntentType::FileRead,
            RiskLevel::L0,
            "developer",
            None,
        );
        let k2 = cache_key_for_step(
            &TaskStep {
                action: "read bar.rs".into(),
                tool: "read_file".into(),
                params: serde_json::json!({}),
                expected_output: None,
            },
            IntentType::FileRead,
            RiskLevel::L0,
            "developer",
            None,
        );
        assert_ne!(key_hash(&k1), key_hash(&k2));
    }

    #[test]
    fn cache_key_hash_ignores_params_byte_changes() {
        let k1 = cache_key_for_step(
            &TaskStep {
                action: "review".into(),
                tool: "read_file".into(),
                params: serde_json::json!({"path": "/tmp/a.rs", "timestamp": 1000}),
                expected_output: None,
            },
            IntentType::CodeReview,
            RiskLevel::L0,
            "developer",
            None,
        );
        let k2 = cache_key_for_step(
            &TaskStep {
                action: "review".into(),
                tool: "read_file".into(),
                params: serde_json::json!({"path": "/tmp/a.rs", "timestamp": 9999}),
                expected_output: None,
            },
            IntentType::CodeReview,
            RiskLevel::L0,
            "developer",
            None,
        );
        assert_eq!(
            key_hash(&k1),
            key_hash(&k2),
            "params 变化不应改变 cache key"
        );
    }
}
