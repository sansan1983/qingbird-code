use crate::common::types::RiskLevel;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use rust_i18n::t;

/// 完整配置结构
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EflowConfig {
    pub core: CoreConfig,
    pub llm: LlmConfig,
    pub memory: MemoryConfig,
    pub security: SecurityConfig,
    pub profiles: ProfileListConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoreConfig {
    pub language: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmConfig {
    pub routing: RoutingConfig,
    #[serde(default)]
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoutingConfig {
    pub strong: String,
    pub medium: String,
    pub light: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_l1_enabled")]
    pub l1_enabled: bool,
    #[serde(default)]
    pub l2_enabled: bool,
    #[serde(default = "default_l2_ttl_days")]
    pub l2_ttl_days: u32,
}

fn default_l1_enabled() -> bool {
    true
}
fn default_l2_ttl_days() -> u32 {
    7
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub working_memory_limit: usize,
    pub project_db_path: String,
    pub user_db_path: String,
    pub cleanup_interval_hours: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub risk_threshold: RiskLevel,
    pub allowed_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileListConfig {
    pub default: String,
    pub available: Vec<String>,
}

/// 加载配置文件
pub fn load_config(path: &Path) -> crate::common::error::Result<EflowConfig> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        crate::common::error::EflowError::Config(
            t!(
                "err_config_read",
                path = path.display().to_string(),
                msg = e.to_string()
            )
            .to_string(),
        )
    })?;

    let expanded = crate::common::env::expand_env_vars(&content)?;

    let config: EflowConfig = serde_yaml::from_str(&expanded).map_err(|e| {
        crate::common::error::EflowError::Config(
            t!("err_config_parse", msg = e.to_string()).to_string(),
        )
    })?;

    Ok(config)
}

/// 寻找配置文件：当前目录 → 用户配置目录 → 系统配置目录
/// 跨平台：Windows 用 %APPDATA%，Unix 用 ~/.config
#[must_use]
pub fn find_config() -> Option<PathBuf> {
    let user_dir = dirs::config_dir().map(|p| p.join("eflow").join("eflow.yaml"));

    let system_dir = if cfg!(windows) {
        // Windows: %PROGRAMDATA%\eflow\eflow.yaml
        std::env::var_os("PROGRAMDATA")
            .map(|p| PathBuf::from(p).join("eflow").join("eflow.yaml"))
    } else {
        // Unix: /etc/eflow/eflow.yaml
        Some(PathBuf::from("/etc/eflow/eflow.yaml"))
    };

    let mut candidates: Vec<PathBuf> = vec![PathBuf::from("eflow.yaml")];
    if let Some(p) = user_dir {
        candidates.push(p);
    }
    if let Some(p) = system_dir {
        candidates.push(p);
    }

    candidates.into_iter().find(|p| p.exists())
}
