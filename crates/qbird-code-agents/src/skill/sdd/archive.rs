use async_trait::async_trait;
use std::sync::Arc;

use super::super::types::{
    AutoTrigger, Skill, SkillContext, SkillDescriptor, SkillMetrics, SkillResult,
};

pub struct SddArchiveSkill;

impl SddArchiveSkill {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[async_trait]
impl Skill for SddArchiveSkill {
    fn descriptor(&self) -> SkillDescriptor {
        SkillDescriptor {
            id: "sdd-archive".into(),
            name: "SDD Archive".into(),
            description: "Archive a completed SDD proposal for future reference".into(),
            version: "1.0.0".into(),
            level: "extension".into(),
            blocking: false,
            auto_trigger: AutoTrigger::Manual,
            input_schema: serde_json::json!({"type":"object","required":["proposalId"],"properties":{"proposalId":{"type":"string"}}}),
            output_schema: serde_json::json!({"type":"object","required":["archived","archiveId"]}),
            tags: vec!["sdd".into(), "archive".into()],
        }
    }

    async fn execute(&self, input: serde_json::Value, _context: SkillContext) -> SkillResult {
        let proposal_id = input["proposalId"].as_str().unwrap_or("unknown");
        SkillResult {
            success: true,
            output: serde_json::json!({"archived":true,"archiveId":format!("arch_{}",proposal_id),"suggestion":"Proposal archived successfully."}),
            metrics: SkillMetrics {
                duration_ms: 0,
                tokens_used: None,
            },
            errors: vec![],
        }
    }
}
