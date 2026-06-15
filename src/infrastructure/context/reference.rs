use uuid::Uuid;

/// 上下文引用指针 — 替代原文进入 LLM 上下文
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextRef {
    pub ref_id: Uuid,
    pub summary: String,
    pub storage_key: String,
    pub token_cost_if_included: u32,
}

impl ContextRef {
    pub fn new(summary: String, storage_key: String, token_cost: u32) -> Self {
        Self {
            ref_id: Uuid::new_v4(),
            summary,
            storage_key,
            token_cost_if_included: token_cost,
        }
    }

    /// 格式化为 LLM 上下文中可识别的引用标记
    pub fn format_for_context(&self) -> String {
        let short_id: String = self.ref_id.to_string().chars().take(8).collect();
        format!("[ref:{}] {}", short_id, self.summary)
    }
}
