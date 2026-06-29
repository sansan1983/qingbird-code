use qbird_code_infra::config::DeepseekConfig;
use qbird_code_infra::runtime_overrides::{CliOverrides, RuntimeOverrides};

fn make_test_config() -> qbird_code_infra::config::EflowConfig {
    let mut cfg = qbird_code_infra::config::EflowConfig::default();
    cfg.llm.active = "deepseek".into();
    cfg.llm.deepseek = DeepseekConfig {
        default_model: "deepseek-v4-pro".into(),
        fast_model: "deepseek-v4-flash".into(),
        ..DeepseekConfig::default()
    };
    cfg.llm.anthropic.default_model = "claude-sonnet-4-6".into();
    cfg
}

#[test]
fn test_set_provider_same_value_preserves_model() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(
        &CliOverrides {
            model: Some("custom-model".into()),
            ..CliOverrides::default()
        },
        &cfg,
    );
    assert_eq!(r.model, Some("custom-model".into()));
    // Setting the same provider should NOT clear model
    r.set_provider("deepseek".into());
    assert_eq!(
        r.model,
        Some("custom-model".into()),
        "same provider should not clear model"
    );
}

#[test]
fn test_resolved_model_fast_alias() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    r.set_model("fast".into());
    assert_eq!(r.resolved_model(&cfg), "deepseek-v4-flash");
}

#[test]
fn test_resolved_model_fast_alias_no_fast_model() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    r.set_provider("anthropic".into());
    r.set_model("fast".into());
    // Anthropic has no fast_model, so "fast" falls back to default_model
    assert_eq!(r.resolved_model(&cfg), "claude-sonnet-4-6");
}

#[test]
fn test_resolved_model_empty_string_uses_default() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    r.set_model("".into());
    assert_eq!(r.resolved_model(&cfg), "deepseek-v4-pro");
}

#[test]
fn test_cli_provider_overrides_config() {
    let cfg = make_test_config();
    let r = RuntimeOverrides::from_cli(
        &CliOverrides {
            provider: Some("anthropic".into()),
            ..CliOverrides::default()
        },
        &cfg,
    );
    assert_eq!(r.provider, "anthropic");
}

#[test]
fn test_default_model_for_unknown_provider() {
    let cfg = make_test_config();
    let r = RuntimeOverrides::from_cli(
        &CliOverrides {
            provider: Some("unknown-provider".into()),
            ..CliOverrides::default()
        },
        &cfg,
    );
    assert_eq!(r.default_model_for(&cfg), "");
}
