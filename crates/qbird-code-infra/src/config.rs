use qbird_code_models::{EflowError, Result, RiskLevel};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ===== 完整配置结构 =====

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EflowConfig {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub profiles: ProfileListConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoreConfig {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_language() -> String {
    "zh-CN".into()
}
fn default_timezone() -> String {
    "Asia/Shanghai".into()
}

// ===== LLM 配置 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// 当前激活的 provider
    #[serde(default = "default_active_provider")]
    pub active: String,
    #[serde(default)]
    pub deepseek: DeepseekConfig,
    #[serde(default)]
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub openai: OpenaiConfig,
    #[serde(default)]
    pub anthropic: AnthropicConfig,
    #[serde(default)]
    pub cache: CacheConfig,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            active: default_active_provider(),
            deepseek: DeepseekConfig::default(),
            ollama: OllamaConfig::default(),
            openai: OpenaiConfig::default(),
            anthropic: AnthropicConfig::default(),
            cache: CacheConfig::default(),
        }
    }
}

fn default_active_provider() -> String {
    "deepseek".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepseekConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_deepseek_base_url")]
    pub base_url: String,
    #[serde(default = "default_deepseek_anthropic_url")]
    pub base_url_anthropic: String,
    #[serde(default = "default_deepseek_model")]
    pub default_model: String,
    #[serde(default = "default_deepseek_fast_model")]
    pub fast_model: String,
    #[serde(default = "default_thinking_enabled")]
    pub thinking_enabled: bool,
    #[serde(default = "default_thinking_effort")]
    pub thinking_effort: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
    #[serde(default)]
    pub cost_per_million_input_tokens: f64,
    #[serde(default)]
    pub cost_per_million_output_tokens: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_ollama_url")]
    pub base_url: String,
    #[serde(default = "default_ollama_model")]
    pub default_model: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
    #[serde(default)]
    pub cost_per_million_input_tokens: f64,
    #[serde(default)]
    pub cost_per_million_output_tokens: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenaiConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_openai_url")]
    pub base_url: String,
    #[serde(default = "default_openai_model")]
    pub default_model: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
    #[serde(default)]
    pub cost_per_million_input_tokens: f64,
    #[serde(default)]
    pub cost_per_million_output_tokens: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_anthropic_url")]
    pub base_url: String,
    #[serde(default = "default_anthropic_model")]
    pub default_model: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
    #[serde(default)]
    pub cost_per_million_input_tokens: f64,
    #[serde(default)]
    pub cost_per_million_output_tokens: f64,
}

// ===== 缓存配置 =====

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_true")]
    pub l1_enabled: bool,
    #[serde(default)]
    pub l2_enabled: bool,
    #[serde(default = "default_7")]
    pub l2_ttl_days: u32,
}

// ===== Memory/Security/Profiles =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_1000")]
    pub working_memory_limit: usize,
    #[serde(default = "default_24")]
    pub cleanup_interval_hours: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            working_memory_limit: default_1000(),
            cleanup_interval_hours: default_24(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default)]
    pub risk_threshold: RiskLevel,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileListConfig {
    #[serde(default)]
    pub default: String,
    #[serde(default)]
    pub available: Vec<String>,
}

// ===== Default Implementations =====

impl Default for DeepseekConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: default_deepseek_base_url(),
            base_url_anthropic: default_deepseek_anthropic_url(),
            default_model: default_deepseek_model(),
            fast_model: default_deepseek_fast_model(),
            thinking_enabled: default_thinking_enabled(),
            thinking_effort: default_thinking_effort(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
            retry_backoff_ms: default_retry_backoff_ms(),
            cost_per_million_input_tokens: 0.0,
            cost_per_million_output_tokens: 0.0,
        }
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: default_ollama_url(),
            default_model: default_ollama_model(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
            retry_backoff_ms: default_retry_backoff_ms(),
            cost_per_million_input_tokens: 0.0,
            cost_per_million_output_tokens: 0.0,
        }
    }
}

impl Default for OpenaiConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: default_openai_url(),
            default_model: default_openai_model(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
            retry_backoff_ms: default_retry_backoff_ms(),
            cost_per_million_input_tokens: 0.0,
            cost_per_million_output_tokens: 0.0,
        }
    }
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: default_anthropic_url(),
            default_model: default_anthropic_model(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
            retry_backoff_ms: default_retry_backoff_ms(),
            cost_per_million_input_tokens: 0.0,
            cost_per_million_output_tokens: 0.0,
        }
    }
}

// ===== 默认值函数 =====

fn default_deepseek_base_url() -> String {
    "https://api.deepseek.com".into()
}
fn default_deepseek_anthropic_url() -> String {
    "https://api.deepseek.com/anthropic".into()
}
fn default_deepseek_model() -> String {
    "deepseek-v4-pro".into()
}
fn default_deepseek_fast_model() -> String {
    "deepseek-v4-flash".into()
}
fn default_ollama_url() -> String {
    "http://localhost:11434".into()
}
fn default_ollama_model() -> String {
    "qwen2.5:14b".into()
}
fn default_openai_url() -> String {
    "https://api.openai.com".into()
}
fn default_openai_model() -> String {
    "gpt-4o".into()
}
fn default_anthropic_url() -> String {
    "https://api.anthropic.com".into()
}
fn default_anthropic_model() -> String {
    "claude-sonnet-4-6".into()
}
fn default_thinking_enabled() -> bool {
    true
}
fn default_thinking_effort() -> String {
    "high".into()
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
fn default_true() -> bool {
    true
}
fn default_7() -> u32 {
    7
}
fn default_1000() -> usize {
    1000
}
fn default_24() -> u64 {
    24
}

// ===== Cost estimation =====

const RMB_RATE: f64 = 7.2;

/// Estimate cost in USD based on token usage and per-million-token pricing.
/// Returns `None` when both cost rates are 0.0 (meaning "unknown").
/// Cache hit tokens are free (already paid for) and excluded from cost.
pub fn estimate_cost(
    input_tokens: u64,
    output_tokens: u64,
    cache_hit_tokens: u64,
    cost_per_million_input: f64,
    cost_per_million_output: f64,
) -> Option<f64> {
    if cost_per_million_input == 0.0 && cost_per_million_output == 0.0 {
        return None;
    }
    let effective_input = input_tokens.saturating_sub(cache_hit_tokens);
    let input_cost = (effective_input as f64 / 1_000_000.0) * cost_per_million_input;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * cost_per_million_output;
    Some(input_cost + output_cost)
}

/// Format estimated cost for display. Returns the i18n key and cost value.
/// For zh-CN locale, converts USD to RMB (hardcoded rate 7.2).
pub fn format_cost(usd_cost: f64, is_zh_locale: bool) -> String {
    if is_zh_locale {
        format!("≈ ¥{:.4}", usd_cost * RMB_RATE)
    } else {
        format!("≈ ${:.4} USD", usd_cost)
    }
}

// ===== 配置加载 =====

pub fn load_config(path: &Path) -> Result<EflowConfig> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| EflowError::Config(format!("读取配置文件失败 {}: {}", path.display(), e)))?;
    let expanded = crate::env::expand_env_vars(&content)?;
    let config: EflowConfig = serde_yaml::from_str(&expanded)
        .map_err(|e| EflowError::Config(format!("解析配置失败: {}", e)))?;
    Ok(config)
}

pub fn find_config() -> Option<PathBuf> {
    let current = PathBuf::from("qingbird.yaml");
    if current.exists() {
        return Some(current);
    }
    let user_dir = dirs::config_dir()
        .or_else(|| {
            if cfg!(windows) {
                std::env::var("APPDATA").ok().map(PathBuf::from)
            } else {
                std::env::var("HOME").ok().map(PathBuf::from)
            }
        })
        .map(|p| p.join("qingbird").join("config.yaml"));
    if let Some(ref p) = user_dir
        && p.exists()
    {
        return Some(p.clone());
    }
    let home_qingbird = dirs::home_dir().map(|p| p.join(".qingbird").join("qingbird.yaml"));
    home_qingbird.filter(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_deepseek_config_uses_v4_pro() {
        let cfg = DeepseekConfig::default();
        assert_eq!(cfg.default_model, "deepseek-v4-pro");
        assert!(cfg.thinking_enabled);
        assert_eq!(cfg.thinking_effort, "high");
    }

    #[test]
    fn default_ollama_config_has_no_key() {
        let cfg = OllamaConfig::default();
        assert!(cfg.api_key.is_none() || cfg.api_key.as_deref() == Some(""));
    }
}
