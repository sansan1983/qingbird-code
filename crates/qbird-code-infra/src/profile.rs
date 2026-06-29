//! User profile support — runtime override files at `<data_dir>/qingbird/profiles/*.yaml`.
//!
//! A `Profile` is loaded from a single yaml file and merged onto the current
//! runtime state (system prompt, allowed tools, risk threshold, provider, model).
//! This is the v0.3.0 (Task 30-02) user-facing "preset" feature: profiles let
//! users save a tuned configuration (e.g. `developer` vs `researcher`) and
//! apply it on demand from CLI or `/profile` slash command.
//!
//! Resolution order (highest priority first):
//! 1. `qingbird --profile <name>` CLI flag
//! 2. `qingbird.yaml` `profiles.default`
//! 3. No profile (use full config defaults)
//!
//! See: `docs/superpowers/plans/2026-06-27-qingbird-v0.3-implementation-plan.md` §Task 30-02.

use std::path::{Path, PathBuf};

use rust_i18n::t;
use serde::{Deserialize, Serialize};

use qbird_code_models::{EflowError, Result};

/// A user profile — loaded from `<profile_dir>/<name>.yaml`.
///
/// Fields are all optional except `name`; missing fields mean "inherit from
/// the underlying config". `merge_into` applies the overrides onto the
/// provided mutable state, following the replace-not-append rule for
/// `system_prompt`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Tool names to whitelist. Empty Vec means "no override" (does not
    /// disable an existing whitelist; see `merge_into`).
    #[serde(default)]
    pub tools_allow: Vec<String>,
    /// `"L0"` .. `"L3"` string. None means inherit.
    #[serde(default)]
    pub risk_threshold: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

impl Profile {
    /// Load a profile from `<profile_dir>/<name>.yaml`.
    ///
    /// Errors:
    /// - `ProfileNotFound { name }` if the file does not exist
    /// - `ProfileMalformed { name, reason }` if the file is unreadable or the
    ///   yaml cannot be parsed (including missing `name` field).
    pub fn load(profile_dir: &Path, name: &str) -> Result<Self> {
        let path = profile_dir.join(format!("{name}.yaml"));
        let text = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(EflowError::ProfileNotFound { name: name.into() });
            }
            Err(e) => {
                return Err(EflowError::ProfileMalformed {
                    name: name.into(),
                    reason: e.to_string(),
                });
            }
        };
        let profile: Profile =
            serde_yaml::from_str(&text).map_err(|e| EflowError::ProfileMalformed {
                name: name.into(),
                reason: e.to_string(),
            })?;
        // Inject the requested name (so a malformed/missing `name` field is
        // caught here rather than surfacing later as a confusing state).
        let mut profile = profile;
        if profile.name != name {
            profile.name = name.to_string();
        }
        Ok(profile)
    }

    /// List all available profile names in `profile_dir`, sorted alphabetically.
    ///
    /// Names are returned without the `.yaml` extension. Returns an empty Vec
    /// if the directory does not exist (not an error — that's a "no profiles
    /// installed" state, distinct from "profiles dir missing because install
    /// is corrupt").
    pub fn list(profile_dir: &Path) -> Result<Vec<String>> {
        if !profile_dir.exists() {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in std::fs::read_dir(profile_dir).map_err(|e| EflowError::ProfileMalformed {
            name: "<list>".into(),
            reason: e.to_string(),
        })? {
            let entry = entry.map_err(|e| EflowError::ProfileMalformed {
                name: "<list>".into(),
                reason: e.to_string(),
            })?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                names.push(stem.to_string());
            }
        }
        names.sort();
        Ok(names)
    }

    /// Create the two sample profiles (`developer` + `researcher`) in
    /// `profile_dir` if it is empty (no `.yaml` files).
    ///
    /// Called on first startup when `Profile::list()` returns empty.
    /// Does nothing if the directory already contains profiles.
    pub fn create_sample_profiles(profile_dir: &Path) -> Result<()> {
        if !profile_dir.exists() {
            std::fs::create_dir_all(profile_dir).map_err(|e| EflowError::ProfileMalformed {
                name: "<create_sample>".into(),
                reason: e.to_string(),
            })?;
        }

        let existing = Self::list(profile_dir)?;
        if !existing.is_empty() {
            return Ok(());
        }

        let developer_yaml = r#"name: developer
description: "Rust development assistant"
system_prompt: "你是一个专业的 Rust 开发助手。使用中文回复，代码注释保持英文。"
tools_allow: []
risk_threshold: L3
thinking_enabled: true
"#;

        let researcher_yaml = r#"name: researcher
description: "Research assistant (read-only)"
system_prompt: "你是一个研究助手，专注于信息检索和分析。只使用只读工具。"
tools_allow:
  - read_file
  - search_code
  - glob
  - list_dir
  - web_fetch
risk_threshold: L1
"#;

        std::fs::write(profile_dir.join("developer.yaml"), developer_yaml).map_err(|e| {
            EflowError::ProfileMalformed {
                name: "developer".into(),
                reason: e.to_string(),
            }
        })?;
        std::fs::write(profile_dir.join("researcher.yaml"), researcher_yaml).map_err(|e| {
            EflowError::ProfileMalformed {
                name: "researcher".into(),
                reason: e.to_string(),
            }
        })?;

        tracing::info!("Created sample profiles: developer, researcher");
        Ok(())
    }

    /// Return the default profile directory: `<data_dir>/qingbird/profiles/`.
    ///
    /// On Windows this is `%APPDATA%\qingbird\profiles\`. On Linux/macOS it
    /// is `$XDG_DATA_HOME/qingbird/profiles/` (or `$HOME/.local/share/...`).
    /// Falls back to a relative `.qingbird/profiles` if `dirs::data_dir()`
    /// returns None (e.g. exotic/embedded environments).
    #[must_use]
    pub fn default_dir() -> PathBuf {
        dirs::data_dir()
            .map(|p| p.join("qingbird").join("profiles"))
            .unwrap_or_else(|| PathBuf::from(".qingbird/profiles"))
    }

    /// Apply this profile onto the current mutable runtime state.
    ///
    /// Replace-not-append rule: `system_prompt`, when present in the profile,
    /// REPLACES the current value (does not append). Provider and model
    /// likewise replace. Risk threshold and allowed tools replace when
    /// present; otherwise inherit.
    ///
    /// **Provider / model require restart** — as of v0.3.0, the LLM
    /// (`HttpLlmClient` + `Box<dyn Provider>`) is constructed at startup
    /// BEFORE the profile is applied (see `main.rs` §4a). The merged
    /// `provider_active` / `model` values flow into local variables that
    /// are then discarded at the call site. Mid-session `/profile <name>`
    /// has the same limitation: the live provider Box never changes.
    ///
    /// To surface this to the user, when the profile carries a `provider`
    /// or `model` value that DIFFERS from the values already in
    /// `provider_active` / `model`, a human-readable warning is pushed
    /// into `warnings` (i18n keys `interactive_profile_warn_provider` /
    /// `interactive_profile_warn_model`). The caller decides whether to
    /// print these to stderr, log them, or surface them in the UI.
    pub fn merge_into(
        &self,
        system_prompt: &mut String,
        allowed_tools: &mut Option<Vec<String>>,
        risk_threshold: &mut Option<String>,
        provider_active: &mut String,
        model: &mut String,
        warnings: &mut Vec<String>,
    ) {
        if let Some(ref sp) = self.system_prompt {
            *system_prompt = sp.clone();
        }
        if !self.tools_allow.is_empty() {
            *allowed_tools = Some(self.tools_allow.clone());
        }
        if let Some(ref r) = self.risk_threshold {
            *risk_threshold = Some(r.clone());
        }
        if let Some(ref p) = self.provider {
            if p != provider_active {
                warnings
                    .push(t!("interactive_profile_warn_provider", value = p.as_str()).to_string());
            }
            *provider_active = p.clone();
        }
        if let Some(ref m) = self.model {
            if m != model {
                warnings.push(t!("interactive_profile_warn_model", value = m.as_str()).to_string());
            }
            *model = m.clone();
        }
    }
}
