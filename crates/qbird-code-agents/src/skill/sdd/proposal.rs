use async_trait::async_trait;
use std::sync::Arc;

use super::super::types::{
    AutoTrigger, SddProposal, Skill, SkillContext, SkillDescriptor, SkillMetrics, SkillResult,
};

pub struct SddProposalSkill;

impl SddProposalSkill {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[async_trait]
impl Skill for SddProposalSkill {
    fn descriptor(&self) -> SkillDescriptor {
        SkillDescriptor {
            id: "sdd-proposal".into(),
            name: "SDD Proposal".into(),
            description: "Generate structured SDD proposal from user requirements with HARD-GATE"
                .into(),
            version: "1.0.0".into(),
            level: "extension".into(),
            blocking: true,
            auto_trigger: AutoTrigger::Conditional,
            input_schema: serde_json::json!({"type":"object","required":["userInput"],"properties":{"userInput":{"type":"string"}}}),
            output_schema: serde_json::json!({"type":"object","required":["proposal","needsReview","hardGateBlocked"]}),
            tags: vec!["sdd".into(), "design".into()],
        }
    }

    async fn execute(&self, input: serde_json::Value, _context: SkillContext) -> SkillResult {
        let user_input = input["userInput"].as_str().unwrap_or("");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let proposal = SddProposal {
            id: format!("p_{}", now),
            goal: user_input.to_string(),
            scope: "new".into(),
            status: "draft".into(),
            hard_gate_blocked: true,
            created_at: now,
            updated_at: now,
        };
        SkillResult {
            success: true,
            output: serde_json::json!({"proposal":proposal,"needsReview":true,"hardGateBlocked":true,"suggestion":"Proposal generated. HARD-GATE blocked - waiting for user confirmation."}),
            metrics: SkillMetrics {
                duration_ms: 0,
                tokens_used: None,
            },
            errors: vec![],
        }
    }
}
