//! yaml 加载 + 与内置 profile 合并。
//!
//! 用户 yaml 格式：
//! ```yaml
//! subagents:
//!   profiles:
//!     general:
//!       prompt_preamble: "..."
//!       max_iterations: 30
//!     my-custom:
//!       prompt_preamble: "..."
//!       tool_policy: readonly
//! ```

use std::collections::HashMap;

use qbird_code_models::{EflowError, Result};
use serde::Deserialize;

use super::profile::{SubagentMode, SubagentProfile, ToolPolicy};

#[derive(Debug, Deserialize)]
struct SubagentsConfig {
    #[serde(default)]
    subagents: SubagentsSection,
}

#[derive(Debug, Default, Deserialize)]
struct SubagentsSection {
    #[serde(default)]
    profiles: Vec<SubagentProfileConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubagentProfileConfig {
    pub name: String,
    pub mode: Option<SubagentMode>,
    pub tool_policy: Option<ToolPolicy>,
    pub prompt_preamble: Option<String>,
    pub description: Option<String>,
    pub default_tools: Option<Vec<String>>,
    pub max_iterations: Option<usize>,
    pub model: Option<String>,
}

/// 从 yaml 文本加载并与内置合并
pub fn load_profiles_from_yaml(
    yaml_text: Option<&str>,
) -> Result<HashMap<String, SubagentProfile>> {
    let mut map: HashMap<String, SubagentProfile> = super::profile::builtin_profiles()
        .into_iter()
        .map(|p| (p.name.clone(), p))
        .collect();

    if let Some(text) = yaml_text {
        let config: SubagentsConfig = serde_yaml::from_str(text)
            .map_err(|e| EflowError::Internal(format!("subagent yaml 解析失败: {}", e)))?;
        merge_into_builtins(&mut map, &config.subagents.profiles);
    }

    Ok(map)
}

/// 顶层加载入口
pub fn load_profiles(yaml_text: Option<&str>) -> Result<HashMap<String, SubagentProfile>> {
    load_profiles_from_yaml(yaml_text)
}

/// 用户配置逐字段覆盖 builtin（None 字段保留 builtin 值）
pub fn merge_into_builtins(
    map: &mut HashMap<String, SubagentProfile>,
    user_configs: &[SubagentProfileConfig],
) {
    for cfg in user_configs {
        if let Some(builtin) = map.get(&cfg.name).cloned() {
            let merged = SubagentProfile {
                name: builtin.name.clone(),
                mode: cfg.mode.unwrap_or(builtin.mode),
                tool_policy: cfg.tool_policy.unwrap_or(builtin.tool_policy),
                prompt_preamble: cfg
                    .prompt_preamble
                    .clone()
                    .unwrap_or(builtin.prompt_preamble),
                description: cfg.description.clone().unwrap_or(builtin.description),
                default_tools: cfg.default_tools.clone().unwrap_or(builtin.default_tools),
                max_iterations: cfg.max_iterations.or(builtin.max_iterations),
                model: cfg.model.clone().or(builtin.model),
            };
            map.insert(cfg.name.clone(), merged);
        } else {
            let new_profile = SubagentProfile {
                name: cfg.name.clone(),
                mode: cfg.mode.unwrap_or(SubagentMode::Subagent),
                tool_policy: cfg.tool_policy.unwrap_or(ToolPolicy::Inherit),
                prompt_preamble: cfg.prompt_preamble.clone().unwrap_or_default(),
                description: cfg.description.clone().unwrap_or_default(),
                default_tools: cfg.default_tools.clone().unwrap_or_default(),
                max_iterations: cfg.max_iterations,
                model: cfg.model.clone(),
            };
            map.insert(cfg.name.clone(), new_profile);
        }
    }
}
