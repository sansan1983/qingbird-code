use serde::Deserialize;

use crate::common::types::RiskLevel;

/// Skill = 可复用的能力单元
#[derive(Debug, Clone, Deserialize)]
pub struct Skill {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub prompt_template: String,
    #[serde(default)]
    pub required_tools: Vec<String>,
}

fn default_version() -> String {
    "1.0".into()
}
