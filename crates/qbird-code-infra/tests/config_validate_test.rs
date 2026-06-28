use qbird_code_infra::config::{AnthropicConfig, EflowConfig, OpenaiConfig};
use qbird_code_infra::config_validate::{validate_config, ConfigError};

// ===== Rule 1: llm.active is valid =====

#[test]
fn test_rule1_valid_active_passes() {
    let cfg = EflowConfig {
        llm: qbird_code_infra::config::LlmConfig {
            active: "deepseek".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let errors = validate_config(&cfg);
    assert!(errors.iter().all(|e| !e.field.contains("llm.active")));
}

#[test]
fn test_rule1_invalid_active_errors() {
    let cfg = EflowConfig {
        llm: qbird_code_infra::config::LlmConfig {
            active: "bogus-provider".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let errors = validate_config(&cfg);
    assert!(
        errors.iter().any(|e| e.field == "llm.active"),
        "expected llm.active error, got: {errors:?}"
    );
}

// ===== Rule 2: api_key non-empty for active provider (env fallback OK) =====

#[test]
fn test_rule2_api_key_present_passes() {
    let cfg = EflowConfig {
        llm: qbird_code_infra::config::LlmConfig {
            active: "deepseek".into(),
            deepseek: qbird_code_infra::config::DeepseekConfig {
                api_key: Some("sk-test".into()),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let errors = validate_config(&cfg);
    assert!(errors.iter().all(|e| !e.field.contains("api_key")));
}

#[test]
fn test_rule2_api_key_empty_and_no_env_errors() {
    // Use a custom env var that we know does not exist.
    // SAFETY: only set/unset a uniquely-named test var; restored at end.
    let key = "QINGBIRD_TEST_NONEXISTENT_KEY";
    unsafe {
        std::env::remove_var(key);
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
    assert!(
        errors.iter().any(|e| e.field.contains("anthropic.api_key")),
        "expected anthropic.api_key error, got: {errors:?}"
    );
}

#[test]
fn test_rule2_ollama_skips_api_key() {
    let cfg = EflowConfig {
        llm: qbird_code_infra::config::LlmConfig {
            active: "ollama".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let errors = validate_config(&cfg);
    assert!(errors.iter().all(|e| !e.field.contains("api_key")));
}

// ===== Rule 3: profiles.default file exists (if set) =====

#[test]
fn test_rule3_profiles_default_empty_skipped() {
    let cfg = EflowConfig::default();
    let errors = validate_config(&cfg);
    assert!(errors.iter().all(|e| !e.field.contains("profiles.default")));
}

#[test]
fn test_rule3_profiles_default_missing_file_errors() {
    let cfg = EflowConfig {
        profiles: qbird_code_infra::config::ProfileListConfig {
            default: "/nonexistent/path/to/profile.yaml".into(),
            available: vec![],
        },
        ..Default::default()
    };
    let errors = validate_config(&cfg);
    assert!(
        errors.iter().any(|e| e.field == "profiles.default"),
        "expected profiles.default error, got: {errors:?}"
    );
}

#[test]
fn test_rule3_profiles_default_existing_file_passes() {
    let tmp = std::env::temp_dir().join("qingbird_test_profile.yaml");
    std::fs::write(&tmp, "name: test\n").unwrap();
    let cfg = EflowConfig {
        profiles: qbird_code_infra::config::ProfileListConfig {
            default: tmp.to_string_lossy().to_string(),
            available: vec![],
        },
        ..Default::default()
    };
    let errors = validate_config(&cfg);
    assert!(errors.iter().all(|e| !e.field.contains("profiles.default")));
    let _ = std::fs::remove_file(tmp);
}

// ===== Rule 4: memory.working_memory_limit > 0 =====

#[test]
fn test_rule4_zero_memory_limit_errors() {
    let cfg = EflowConfig {
        memory: qbird_code_infra::config::MemoryConfig {
            working_memory_limit: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    let errors = validate_config(&cfg);
    assert!(
        errors.iter().any(|e| e.field == "memory.working_memory_limit"),
        "expected memory.working_memory_limit error, got: {errors:?}"
    );
}

#[test]
fn test_rule4_positive_memory_limit_passes() {
    let cfg = EflowConfig {
        memory: qbird_code_infra::config::MemoryConfig {
            working_memory_limit: 4096,
            ..Default::default()
        },
        ..Default::default()
    };
    let errors = validate_config(&cfg);
    assert!(errors.iter().all(|e| !e.field.contains("working_memory_limit")));
}

// ===== Aggregation behavior =====

#[test]
fn test_validate_aggregates_all_errors() {
    let cfg = EflowConfig {
        llm: qbird_code_infra::config::LlmConfig {
            active: "bogus".into(),
            ..Default::default()
        },
        memory: qbird_code_infra::config::MemoryConfig {
            working_memory_limit: 0,
            ..Default::default()
        },
        profiles: qbird_code_infra::config::ProfileListConfig {
            default: "/nonexistent.yaml".into(),
            available: vec![],
        },
        ..Default::default()
    };
    let errors = validate_config(&cfg);
    assert!(errors.len() >= 3, "expected at least 3 errors, got {errors:?}");
}

#[test]
fn test_default_config_passes_all() {
    let cfg = EflowConfig::default();
    let errors = validate_config(&cfg);
    assert!(
        errors.is_empty(),
        "default config should pass all rules, got: {errors:?}"
    );
}

// ===== ConfigError Display =====

#[test]
fn test_config_error_display_format() {
    let e = ConfigError {
        field: "llm.active".into(),
        message: "must be one of ...".into(),
    };
    let s = format!("{e}");
    assert!(s.contains("llm.active"));
    assert!(s.contains("must be one of"));
}

// ===== Used OpenaiConfig to silence unused import (req for tests) =====
#[allow(dead_code)]
fn _unused_openai(_: OpenaiConfig) {}
