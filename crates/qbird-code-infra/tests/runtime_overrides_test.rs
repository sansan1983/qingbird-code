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
fn test_overrides_from_cli_priority() {
    let cfg = make_test_config();
    // CLI --provider wins over cfg.llm.active
    let cli = CliOverrides {
        provider: Some("anthropic".into()),
        model: Some("custom-model".into()),
        temperature: Some(0.5),
    };
    let r = RuntimeOverrides::from_cli(&cli, &cfg);
    assert_eq!(r.provider, "anthropic");
    assert_eq!(r.model.as_deref(), Some("custom-model"));
    assert_eq!(r.temperature, Some(0.5));
}

#[test]
fn test_overrides_from_cli_falls_back_to_cfg() {
    let cfg = make_test_config();
    let cli = CliOverrides::default();
    let r = RuntimeOverrides::from_cli(&cli, &cfg);
    assert_eq!(r.provider, "deepseek"); // from cfg.llm.active
    assert_eq!(r.model, None);
    assert_eq!(r.temperature, None);
}

#[test]
fn test_set_provider_clears_model() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    r.set_model("deepseek-v4-pro".into());
    assert!(r.model.is_some());
    r.set_provider("anthropic".into());
    assert_eq!(r.provider, "anthropic");
    assert_eq!(r.model, None, "switching provider must reset model");
}

#[test]
fn test_set_provider_same_name_keeps_model() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    r.set_model("deepseek-v4-pro".into());
    r.set_provider("deepseek".into()); // same as current
    assert_eq!(r.model.as_deref(), Some("deepseek-v4-pro"));
}

#[test]
fn test_set_model_does_not_persist_to_cfg() {
    let cfg = make_test_config();
    let original = cfg.llm.deepseek.default_model.clone();
    let mut r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    r.set_model("totally-different-model".into());
    assert_eq!(r.model.as_deref(), Some("totally-different-model"));
    assert_eq!(cfg.llm.deepseek.default_model, original); // cfg unchanged
}

#[test]
fn test_set_provider_does_not_mutate_cfg() {
    let cfg = make_test_config();
    let original_provider = cfg.llm.active.clone();
    let mut r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    r.set_provider("anthropic".into());
    assert_eq!(r.provider, "anthropic");
    assert_eq!(cfg.llm.active, original_provider); // cfg unchanged
}

#[test]
fn test_temperature_independent_of_provider() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(
        &CliOverrides {
            temperature: Some(0.9),
            ..CliOverrides::default()
        },
        &cfg,
    );
    r.set_provider("anthropic".into());
    assert_eq!(r.temperature, Some(0.9));
    r.set_temperature(0.3);
    assert_eq!(r.provider, "anthropic");
    assert_eq!(r.temperature, Some(0.3));
}

#[test]
fn test_resolved_model_default() {
    let cfg = make_test_config();
    let r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    assert_eq!(r.resolved_model(&cfg), "deepseek-v4-pro");
}

#[test]
fn test_resolved_model_explicit() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    r.set_model("custom-v5".into());
    assert_eq!(r.resolved_model(&cfg), "custom-v5");
}

#[test]
fn test_resolved_model_fast_alias() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    r.set_model("fast".into());
    assert_eq!(r.resolved_model(&cfg), "deepseek-v4-flash");
}

#[test]
fn test_resolved_model_fast_alias_anthropic_empty() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    r.set_provider("anthropic".into());
    r.set_model("fast".into());
    // anthropic has no fast_model — falls back to default_model
    assert_eq!(r.resolved_model(&cfg), "claude-sonnet-4-6");
}
