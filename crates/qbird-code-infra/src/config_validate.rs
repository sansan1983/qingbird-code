//! EflowConfig validation rules.
//!
//! Each rule produces a `ConfigError` (field + i18n message). The validator
//! returns ALL errors (does not stop at the first), so users can fix multiple
//! issues per run.

use std::fmt;

use crate::config::EflowConfig;
use crate::profile::Profile;

const VALID_PROVIDERS: &[&str] = &[
    "deepseek",
    "deepseek-anthropic",
    "ollama",
    "openai",
    "anthropic",
];

/// A single configuration validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    /// Field path in dot notation, e.g. `llm.active`, `llm.deepseek.api_key`.
    pub field: String,
    /// Pre-localized user-facing message (already passed through `t!()`).
    pub message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.field, self.message)
    }
}

impl std::error::Error for ConfigError {}

/// Validate an `EflowConfig` and return ALL errors found (empty Vec = OK).
///
/// Rules:
/// 1. `llm.active` is one of the known provider names.
/// 2. For the active provider, `api_key` is set OR the env-var fallback exists
///    (Ollama is exempt — local).
/// 3. `profiles.default`, if non-empty, points to an existing file.
/// 4. `memory.working_memory_limit` is greater than 0.
///
/// Notes:
/// - `security.allowed_paths` is intentionally NOT checked: its default
///   (empty Vec) means "allow all" (ToolRegistry behaviour at
///   `tools/src/registry.rs:93`), which is a safe default. A non-empty
///   value is opt-in user restriction. Adding a "must be non-empty" rule
///   here would break setups that rely on the default.
/// - `cost_per_million_*_tokens` is NOT checked: those fields are added
///   in task 30-04 and are out of scope here.
#[must_use]
pub fn validate_config(cfg: &EflowConfig) -> Vec<ConfigError> {
    let mut errors = Vec::new();
    check_active_provider(cfg, &mut errors);
    check_api_key(cfg, &mut errors);
    check_profiles_default(cfg, &mut errors);
    check_memory_limit(cfg, &mut errors);
    errors
}

fn check_active_provider(cfg: &EflowConfig, errors: &mut Vec<ConfigError>) {
    if VALID_PROVIDERS.contains(&cfg.llm.active.as_str()) {
        return;
    }
    let valid = VALID_PROVIDERS.join("/");
    let msg = rust_i18n::t!(
        "cfg_err_invalid_active",
        value = cfg.llm.active.as_str(),
        valid = valid.as_str()
    )
    .into_owned();
    errors.push(ConfigError {
        field: "llm.active".into(),
        message: msg,
    });
}

fn check_api_key(cfg: &EflowConfig, errors: &mut Vec<ConfigError>) {
    let active = cfg.llm.active.as_str();
    if active.is_empty() || active == "ollama" {
        return;
    }
    let (key_empty, env_var) = match active {
        "deepseek" | "deepseek-anthropic" => (
            cfg.llm
                .deepseek
                .api_key
                .as_deref()
                .is_none_or(str::is_empty),
            "DEEPSEEK_API_KEY",
        ),
        "openai" => (
            cfg.llm.openai.api_key.as_deref().is_none_or(str::is_empty),
            "OPENAI_API_KEY",
        ),
        "anthropic" => (
            cfg.llm
                .anthropic
                .api_key
                .as_deref()
                .is_none_or(str::is_empty),
            "ANTHROPIC_API_KEY",
        ),
        _ => return,
    };
    if !key_empty {
        return;
    }
    if std::env::var(env_var).is_ok() {
        return;
    }
    let msg = rust_i18n::t!("cfg_err_api_key_missing", env_var = env_var).into_owned();
    errors.push(ConfigError {
        field: format!("llm.{active}.api_key"),
        message: msg,
    });
}

fn check_profiles_default(cfg: &EflowConfig, errors: &mut Vec<ConfigError>) {
    if cfg.profiles.default.is_empty() {
        return;
    }
    let profile_path = Profile::default_dir().join(format!("{}.yaml", cfg.profiles.default));
    if profile_path.exists() {
        return;
    }
    let msg = rust_i18n::t!(
        "cfg_err_profile_not_found",
        path = cfg.profiles.default.as_str()
    )
    .into_owned();
    errors.push(ConfigError {
        field: "profiles.default".into(),
        message: msg,
    });
}

fn check_memory_limit(cfg: &EflowConfig, errors: &mut Vec<ConfigError>) {
    if cfg.memory.working_memory_limit > 0 {
        return;
    }
    let msg = rust_i18n::t!("cfg_err_zero_mem_limit").into_owned();
    errors.push(ConfigError {
        field: "memory.working_memory_limit".into(),
        message: msg,
    });
}
