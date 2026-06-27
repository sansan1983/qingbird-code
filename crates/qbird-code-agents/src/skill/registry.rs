use std::collections::HashMap;
use std::sync::Arc;

use super::types::{AutoTrigger, Skill, SkillContext, SkillDescriptor, SkillResult};
use qbird_code_models::EflowError;

pub struct SkillRegistry {
    skills: HashMap<String, Arc<dyn Skill>>,
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    pub fn register(&mut self, skill: Arc<dyn Skill>) {
        let id = skill.descriptor().id.clone();
        self.skills.insert(id, skill);
    }

    pub fn get(&self, id: &str) -> Option<&Arc<dyn Skill>> {
        self.skills.get(id)
    }

    pub fn list(&self) -> Vec<SkillDescriptor> {
        let mut all: Vec<SkillDescriptor> = self.skills.values().map(|s| s.descriptor()).collect();
        all.sort_by(|a, b| a.level.cmp(&b.level).then(a.id.cmp(&b.id)));
        all
    }

    pub fn match_auto(&self) -> Vec<SkillDescriptor> {
        self.skills
            .values()
            .filter(|s| s.descriptor().auto_trigger == AutoTrigger::Always)
            .map(|s| s.descriptor())
            .collect()
    }

    pub async fn execute(
        &self,
        id: &str,
        input: serde_json::Value,
        context: SkillContext,
    ) -> Result<SkillResult, EflowError> {
        let skill = self
            .skills
            .get(id)
            .ok_or_else(|| EflowError::SkillNotFound(id.to_string()))?;
        Ok(skill.execute(input, context).await)
    }
}
