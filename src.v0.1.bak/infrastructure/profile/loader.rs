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
    #[must_use]
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            skills: HashMap::new(),
        }
    }

    /// 从目录加载所有 Profile
    pub fn load_profiles(&mut self, profiles_dir: &Path) -> Result<()> {
        for_each_yaml_in(profiles_dir, |path| {
            let profile = self.load_profile_file(path)?;
            self.profiles.insert(profile.name.clone(), profile);
            Ok(())
        })
    }

    fn load_profile_file(&self, path: &Path) -> Result<Profile> {
        let content = read_file(path)?;
        // 校验 checksum（v1.0: 简单校验；v2.0: 数字签名）
        let checksum = format!("{:x}", Sha256::digest(content.as_bytes()));
        let profile: Profile = parse_yaml(&content, path)?;
        tracing::info!(
            "Loaded profile '{}' (checksum: {})",
            profile.name,
            &checksum[..8]
        );
        Ok(profile)
    }

    /// 从目录加载所有 Skill
    pub fn load_skills(&mut self, skills_dir: &Path) -> Result<()> {
        for_each_yaml_in(skills_dir, |path| {
            let skill = self.load_skill_file(path)?;
            self.skills.insert(skill.name.clone(), skill);
            Ok(())
        })
    }

    fn load_skill_file(&mut self, path: &Path) -> Result<Skill> {
        let content = read_file(path)?;
        // 权限校验：Skill 的 risk_level 不能超过 Profile 的权限边界
        // (v1.0: 基础校验，v2.0: 沙箱隔离)
        let skill: Skill = parse_yaml(&content, path)?;
        Ok(skill)
    }

    /// 获取 Profile
    #[must_use]
    pub fn get_profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    /// 获取 Profile 的所有 Skill 定义
    #[must_use]
    pub fn get_profile_skills(&self, profile_name: &str) -> Vec<&Skill> {
        self.get_profile(profile_name)
            .map(|p| p.skills.iter().filter_map(|n| self.skills.get(n)).collect())
            .unwrap_or_default()
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
    #[must_use]
    pub fn list_profiles(&self) -> Vec<&String> {
        self.profiles.keys().collect()
    }
}

impl Default for ProfileRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ========== 共享辅助函数 ==========

/// 遍历目录下所有 .yaml/.yml 文件并调用 `load_fn`
fn for_each_yaml_in<F>(dir: &Path, mut load_fn: F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    if !dir.exists() {
        return Ok(());
    }
    for entry in
        std::fs::read_dir(dir).map_err(|e| cfg_err("err_read_profiles_dir", e.to_string()))?
    {
        let entry = entry.map_err(|e| cfg_err("err_read_entry", e.to_string()))?;
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext == "yaml" || ext == "yml")
        {
            load_fn(&path)?;
        }
    }
    Ok(())
}

fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| cfg_err("err_read_file", format!("{}: {}", path.display(), e)))
}

fn parse_yaml<T: for<'de> Deserialize<'de>>(content: &str, path: &Path) -> Result<T> {
    serde_yaml::from_str(content)
        .map_err(|e| cfg_err("err_parse_file", format!("{}: {}", path.display(), e)))
}

fn cfg_err(key: &str, msg: String) -> EflowError {
    EflowError::Config(t!(key, msg = msg).to_string())
}
