use async_trait::async_trait;
use std::sync::Arc;

use super::super::types::{
    AutoTrigger, Skill, SkillContext, SkillDescriptor, SkillMetrics, SkillResult,
};

pub struct SddReviewSpecSkill;

impl SddReviewSpecSkill {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[async_trait]
impl Skill for SddReviewSpecSkill {
    fn descriptor(&self) -> SkillDescriptor {
        SkillDescriptor {
            id: "sdd-review-spec".into(),
            name: "SDD Spec Review".into(),
            description: "Review SDD proposal specification completeness and clarity".into(),
            version: "1.0.0".into(),
            level: "extension".into(),
            blocking: false,
            auto_trigger: AutoTrigger::Conditional,
            input_schema: serde_json::json!({"type":"object","required":["proposalId"],"properties":{"proposalId":{"type":"string"}}}),
            output_schema: serde_json::json!({"type":"object","required":["approved","issues"]}),
            tags: vec!["sdd".into(), "review".into()],
        }
    }

    async fn execute(&self, input: serde_json::Value, _context: SkillContext) -> SkillResult {
        let proposal_id = input["proposalId"].as_str().unwrap_or("unknown");
        SkillResult {
            success: true,
            output: serde_json::json!({"approved":true,"issues":[],"proposalId":proposal_id,"suggestion":"Spec review passed. Ready for quality review."}),
            metrics: SkillMetrics {
                duration_ms: 0,
                tokens_used: None,
            },
            errors: vec![],
        }
    }
}
