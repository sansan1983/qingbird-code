//! Cache 类型定义
//!
//! 子模块说明：
//! - `CacheKey` / `ContextProfile` / `CacheValue` — 数据定义（本文件）
//! - `cache_key_for_step` / `key_hash` / `intent_label` — cache_key.rs
//! - `MemoryLruBackend` — l1.rs
//! - `SqliteCacheBackend` / `L2CacheManager` / `CacheStats` / `unix_now` — l2.rs

use serde::{Deserialize, Serialize};

use crate::common::types::{IntentType, RiskLevel};

/// L2 缓存 Key（设计 §8.3）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    pub intent_type: IntentType,
    pub task_signature: String,
    pub context_profile: ContextProfile,
    pub model: String,
}

/// 上下文特征（不存具体内容，存 bucket）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextProfile {
    /// 会话长度分桶：0=0-5, 1=6-20, 2=21-100, 3=100+
    pub conversation_depth_bucket: u8,
    /// 涉及文件数分桶：0=0, 1=1-3, 2=4-10, 3=10+
    pub file_count_bucket: u8,
    pub risk_level: RiskLevel,
    pub profile_name: String,
}

/// 缓存值（设计 §8.4）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheValue {
    Decision {
        plan_summary: String,
        risk: RiskLevel,
        model_choice: String,
    },
    Execution {
        result_summary: String,
        success: bool,
        duration_ms: u64,
    },
    Feedback {
        verdict_summary: String,
        confidence: f32,
    },
}
