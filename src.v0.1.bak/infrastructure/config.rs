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
    #[serde(default)]
    pub cache: CacheConfig,
    /// V0.1.0: DeepSeek 专属配置
    #[serde(default)]
    pub deepseek: DeepseekConfig,
}

/// DeepSeek 提供商配置 — V0.1.0 deepseek-only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepseekConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
}

impl Default for DeepseekConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: None,
            default_model: None,
            timeout_secs: 30,
            max_retries: 3,
            retry_backoff_ms: 1000,
        }
    }
}

fn default_timeout_secs() -> u64 {
    30
}
fn default_max_retries() -> u8 {
    3
}
fn default_retry_backoff_ms() -> u64 {
    1000
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

/// 寻找配置文件：当前目录 → 用户配置目录
/// 优先读当前目录的 `qingbird.yaml`，回退 `~/.qingbird/config.yaml`
#[must_use]
pub fn find_config() -> Option<PathBuf> {
    let current = PathBuf::from("qingbird.yaml");
    if current.exists() {
        return Some(current);
    }

    let user_dir = dirs::config_dir()
        .or_else(|| {
            // fallback: 检查 HOME/APPDATA 层级
            if cfg!(windows) {
                let p = PathBuf::from(std::env::var("APPDATA").unwrap_or_default());
                (!p.as_os_str().is_empty()).then_some(p)
            } else {
                std::env::var("HOME").ok().map(PathBuf::from)
            }
        })
        .map(|p| p.join("qingbird").join("config.yaml"));

    if let Some(p) = user_dir.filter(|p| p.exists()) {
        return Some(p);
    }

    None
}
