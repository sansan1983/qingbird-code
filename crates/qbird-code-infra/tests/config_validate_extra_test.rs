use qbird_code_infra::config::{AnthropicConfig, EflowConfig};
use qbird_code_infra::config_validate::{ConfigError, validate_config};

#[test]
fn test_rule4_negative_memory_limit_errors() {
    // working_memory_limit is usize so negative isn't possible via serde,
    // but zero is the boundary case — already tested. This tests that
    // a very small positive value (1) passes.
    let cfg = EflowConfig {
        memory: qbird_code_infra::config::MemoryConfig {
            working_memory_limit: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    let errors = validate_config(&cfg);
    assert!(
        errors
            .iter()
            .all(|e| !e.field.contains("working_memory_limit")),
        "limit=1 should pass, got: {errors:?}"
    );
}

#[test]
fn test_validate_all_rules_pass_with_valid_config() {
    let key = "DEEPSEEK_API_KEY";
    let original = std::env::var(key).ok();
    unsafe {
        std::env::set_var(key, "sk-test-dummy");
    }
    let cfg = EflowConfig {
        llm: qbird_code_infra::config::LlmConfig {
            active: "deepseek".into(),
            deepseek: qbird_code_infra::config::DeepseekConfig {
                api_key: Some("sk-test".into()),
                ..Default::default()
            },
            ..Default::default()
        },
        memory: qbird_code_infra::config::MemoryConfig {
            working_memory_limit: 1000,
            ..Default::default()
        },
        ..Default::default()
    };
    let errors = validate_config(&cfg);
    match original {
        Some(v) => unsafe { std::env::set_var(key, v) },
        None => unsafe { std::env::remove_var(key) },
    }
    assert!(
        errors.is_empty(),
        "valid config should have no errors, got: {errors:?}"
    );
}

#[test]
fn test_rule2_env_var_fallback_anthropic() {
    // When api_key is empty but ANTHROPIC_API_KEY env var exists, it should pass.
    let key = "ANTHROPIC_API_KEY";
    let original = std::env::var(key).ok();
    unsafe {
        std::env::set_var(key, "sk-from-env");
    }
    let cfg = EflowConfig {
        llm: qbird_code_infra::config::LlmConfig {
            active: "anthropic".into(),
            anthropic: AnthropicConfig {
                api_key: Some(String::new()),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let errors = validate_config(&cfg);
    match original {
        Some(v) => unsafe { std::env::set_var(key, v) },
        None => unsafe { std::env::remove_var(key) },
    }
    assert!(
        errors.iter().all(|e| !e.field.contains("api_key")),
        "env var fallback should satisfy rule 2, got: {errors:?}"
    );
}

#[test]
fn test_config_error_display_contains_field_and_message() {
    let e = ConfigError {
        field: "memory.working_memory_limit".into(),
        message: "must be > 0".into(),
    };
    let s = format!("{e}");
    assert!(s.contains("memory.working_memory_limit"));
    assert!(s.contains("must be > 0"));
}

#[test]
fn test_validate_multiple_errors_returns_all() {
    let cfg = EflowConfig {
        llm: qbird_code_infra::config::LlmConfig {
            active: "nonexistent".into(),
            ..Default::default()
        },
        memory: qbird_code_infra::config::MemoryConfig {
            working_memory_limit: 0,
            ..Default::default()
        },
        profiles: qbird_code_infra::config::ProfileListConfig {
            default: "/no/such/file.yaml".into(),
            available: vec![],
        },
        ..Default::default()
    };
    let errors = validate_config(&cfg);
    // Should have at least: llm.active, memory.working_memory_limit, profiles.default
    assert!(
        errors.len() >= 3,
        "expected at least 3 aggregated errors, got {}: {errors:?}",
        errors.len()
    );
    let fields: Vec<&str> = errors.iter().map(|e| e.field.as_str()).collect();
    assert!(fields.contains(&"llm.active"), "missing llm.active error");
    assert!(
        fields.contains(&"memory.working_memory_limit"),
        "missing memory error"
    );
    assert!(
        fields.contains(&"profiles.default"),
        "missing profiles.default error"
    );
}
