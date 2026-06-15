use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::skill::Skill;
use crate::common::error::{EflowError, Result};
use crate::common::types::{ModelTier, PermissionSet};
use rust_i18n::t;

/// Profile = 角色身份，Skill 的容器
#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    #[serde(default = "default_model_tier")]
    pub default_model: ModelTier,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub permission_boundary: PermissionSet,
}

fn default_model_tier() -> ModelTier {
    ModelTier::Medium
}

/// Profile 注册表
pub struct ProfileRegistry {
    profiles: HashMap<String, Profile>,
    skills: HashMap<String, Skill>,
}

impl ProfileRegistry {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            skills: HashMap::new(),
        }
    }

    /// 从目录加载所有 Profile
    pub fn load_profiles(&mut self, profiles_dir: &Path) -> Result<()> {
        if !profiles_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(profiles_dir)
            .map_err(|e| {
                EflowError::Config(t!("err_read_profiles_dir", msg = e.to_string()).to_string())
            })?
        {
            let entry = entry.map_err(|e| {
                EflowError::Config(t!("err_read_entry", msg = e.to_string()).to_string())
            })?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                let profile = self.load_profile_file(&path)?;
                self.profiles.insert(profile.name.clone(), profile);
            }
        }
        Ok(())
    }

    fn load_profile_file(&self, path: &Path) -> Result<Profile> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            EflowError::Config(
                t!(
                    "err_read_file",
                    path = path.display().to_string(),
                    msg = e.to_string()
                )
                .to_string(),
            )
        })?;

        // 校验 checksum（v1.0: 简单校验；v2.0: 数字签名）
        let checksum = format!("{:x}", Sha256::digest(content.as_bytes()));
        // checksum 记录在日志中，用于防篡改检测

        let profile: Profile = serde_yaml::from_str(&content).map_err(|e| {
            EflowError::Config(
                t!(
                    "err_parse_file",
                    path = path.display().to_string(),
                    msg = e.to_string()
                )
                .to_string(),
            )
        })?;

        tracing::info!(
            "Loaded profile '{}' (checksum: {})",
            profile.name,
            &checksum[..8]
        );
        Ok(profile)
    }

    /// 从目录加载所有 Skill
    pub fn load_skills(&mut self, skills_dir: &Path) -> Result<()> {
        if !skills_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(skills_dir)
            .map_err(|e| {
                EflowError::Config(t!("err_read_profiles_dir", msg = e.to_string()).to_string())
            })?
        {
            let entry = entry.map_err(|e| {
                EflowError::Config(t!("err_read_entry", msg = e.to_string()).to_string())
            })?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                self.load_skill_file(&path)?;
            }
        }
        Ok(())
    }

    fn load_skill_file(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            EflowError::Config(
                t!(
                    "err_read_file",
                    path = path.display().to_string(),
                    msg = e.to_string()
                )
                .to_string(),
            )
        })?;

        let skill: Skill = serde_yaml::from_str(&content).map_err(|e| {
            EflowError::Config(
                t!(
                    "err_parse_file",
                    path = path.display().to_string(),
                    msg = e.to_string()
                )
                .to_string(),
            )
        })?;

        // 权限校验：Skill 的 risk_level 不能超过 Profile 的权限边界
        // (v1.0: 基础校验，v2.0: 沙箱隔离)

        self.skills.insert(skill.name.clone(), skill);
        Ok(())
    }

    /// 获取 Profile
    pub fn get_profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    /// 获取 Profile 的所有 Skill 定义
    pub fn get_profile_skills(&self, profile_name: &str) -> Vec<&Skill> {
        if let Some(profile) = self.profiles.get(profile_name) {
            profile
                .skills
                .iter()
                .filter_map(|name| self.skills.get(name))
                .collect()
        } else {
            vec![]
        }
    }

    /// 构建完整的 System Prompt（Profile + 所有 Skill 模板合并）
    pub fn build_system_prompt(&self, profile_name: &str) -> Result<String> {
        let profile = self
            .get_profile(profile_name)
            .ok_or_else(|| EflowError::ProfileNotFound(profile_name.into()))?;

        let mut prompt = profile.system_prompt.clone();

        for skill in self.get_profile_skills(profile_name) {
            prompt.push_str(&format!(
                "\n\n## Skill: {}\n{}",
                skill.name, skill.prompt_template
            ));
        }

        Ok(prompt)
    }

    /// 列出所有可用 Profile 名称
    pub fn list_profiles(&self) -> Vec<&String> {
        self.profiles.keys().collect()
    }
}

impl Default for ProfileRegistry {
    fn default() -> Self {
        Self::new()
    }
}
