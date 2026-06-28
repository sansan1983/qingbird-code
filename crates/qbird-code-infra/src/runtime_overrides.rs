//! Runtime overrides for LLM call: provider, model, temperature.
//!
//! Resolved at startup from CLI flags; mutated by slash commands during REPL.
//! Does NOT mutate `EflowConfig` — runtime state is independent of yaml,
//! so switching providers does not pollute the loaded config or future runs.
//!
//! Priority chain (highest first):
//!   CLI flag → cfg.llm.<provider>.default_model → alias resolution (`fast` → fast_model)

use crate::config::EflowConfig;

/// Runtime overrides for the current session.
#[derive(Debug, Clone)]
pub struct RuntimeOverrides {
    /// Active provider name (e.g. "deepseek", "anthropic").
    pub provider: String,
    /// Explicit model override; `None` means use provider's `default_model`.
    /// The literal string `"fast"` is resolved to the provider's `fast_model`
    /// (currently only DeepSeek has one).
    pub model: Option<String>,
    /// Explicit temperature override; `None` means use provider's default.
    pub temperature: Option<f64>,
}

/// CLI inputs needed to construct initial `RuntimeOverrides`.
#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
}

impl RuntimeOverrides {
    /// Build initial overrides from CLI inputs and loaded config.
    /// CLI `--provider` wins over `cfg.llm.active`.
    pub fn from_cli(cli: &CliOverrides, cfg: &EflowConfig) -> Self {
        let provider = cli
            .provider
            .clone()
            .unwrap_or_else(|| cfg.llm.active.clone());
        Self {
            provider,
            model: cli.model.clone(),
            temperature: cli.temperature,
        }
    }

    /// Switch to a different provider. Resets `model` to `None` so the
    /// new provider's `default_model` takes effect (avoids passing
    /// `"deepseek-v4-pro"` to Anthropic, etc.).
    pub fn set_provider(&mut self, name: String) {
        if self.provider != name {
            self.model = None;
        }
        self.provider = name;
    }

    /// Set explicit model. Use the literal `"fast"` to alias to the
    /// provider's `fast_model` (DeepSeek only).
    pub fn set_model(&mut self, name: String) {
        self.model = Some(name);
    }

    /// Set explicit temperature.
    pub fn set_temperature(&mut self, value: f64) {
        self.temperature = Some(value);
    }

    /// Resolve the model name to use, applying alias resolution.
    /// Returns a `String` (owned) because alias expansion may copy `fast_model`.
    /// The `"fast"` alias resolves to `fast_model`; if the provider has none
    /// (e.g. Anthropic), falls back to `default_model`.
    #[must_use]
    pub fn resolved_model(&self, cfg: &EflowConfig) -> String {
        match self.model.as_deref() {
            None | Some("") => self.default_model_for(cfg).to_string(),
            Some("fast") => {
                let fast = self.fast_model_for(cfg);
                if fast.is_empty() {
                    self.default_model_for(cfg).to_string()
                } else {
                    fast.to_string()
                }
            }
            Some(name) => name.to_string(),
        }
    }

    /// Default model for the active provider.
    #[must_use]
    pub fn default_model_for<'a>(&self, cfg: &'a EflowConfig) -> &'a str {
        match self.provider.as_str() {
            "deepseek" | "deepseek-anthropic" => &cfg.llm.deepseek.default_model,
            "ollama" => &cfg.llm.ollama.default_model,
            "openai" => &cfg.llm.openai.default_model,
            "anthropic" => &cfg.llm.anthropic.default_model,
            _ => "",
        }
    }

    /// Fast model for the active provider; empty string if not configured.
    #[must_use]
    pub fn fast_model_for<'a>(&self, cfg: &'a EflowConfig) -> &'a str {
        match self.provider.as_str() {
            "deepseek" | "deepseek-anthropic" => &cfg.llm.deepseek.fast_model,
            _ => "",
        }
    }

    /// Current provider name.
    #[must_use]
    pub fn current_provider(&self) -> &str {
        &self.provider
    }
}
