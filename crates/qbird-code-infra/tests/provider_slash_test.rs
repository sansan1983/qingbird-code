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
fn test_provider_slash_switch() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    assert_eq!(r.current_provider(), "deepseek");
    r.set_provider("ollama".into());
    assert_eq!(r.current_provider(), "ollama");
}

#[test]
fn test_provider_slash_clears_model() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    r.set_model("deepseek-v4-pro".into());
    assert!(r.model.is_some());
    r.set_provider("anthropic".into());
    assert_eq!(r.model, None, "switching provider must clear model");
    assert_eq!(r.resolved_model(&cfg), "claude-sonnet-4-6");
}

#[test]
fn test_provider_slash_invalid_name() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(&CliOverrides::default(), &cfg);
    // RuntimeOverrides does not validate provider names; validation
    // happens at LLM init time in main.rs match block.
    r.set_provider("totally-unknown-provider".into());
    assert_eq!(r.current_provider(), "totally-unknown-provider");
}

#[test]
fn test_provider_slash_preserves_temperature() {
    let cfg = make_test_config();
    let mut r = RuntimeOverrides::from_cli(
        &CliOverrides {
            temperature: Some(0.7),
            ..CliOverrides::default()
        },
        &cfg,
    );
    assert_eq!(r.temperature, Some(0.7));
    r.set_provider("ollama".into());
    assert_eq!(
        r.temperature,
        Some(0.7),
        "temperature must survive provider switch"
    );
}
