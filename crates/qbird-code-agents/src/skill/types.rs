use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoTrigger {
    Always,
    Conditional,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub level: String,
    pub blocking: bool,
    pub auto_trigger: AutoTrigger,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SkillContext {
    pub session_id: String,
    pub project_path: Option<String>,
    pub budget_remaining: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub metrics: SkillMetrics,
    pub errors: Vec<SkillError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetrics {
    pub duration_ms: u64,
    pub tokens_used: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillError {
    pub code: String,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SddProposal {
    pub id: String,
    pub goal: String,
    pub scope: String,
    pub status: String,
    pub hard_gate_blocked: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[async_trait::async_trait]
pub trait Skill: Send + Sync {
    fn descriptor(&self) -> SkillDescriptor;
    async fn execute(&self, input: serde_json::Value, context: SkillContext) -> SkillResult;
}
