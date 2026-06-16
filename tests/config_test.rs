rust_i18n::i18n!("locales", fallback = "en-US");

use eflow::infrastructure::config::load_config;
use std::io::Write;

#[test]
fn test_load_valid_config() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    let yaml = r#"
core:
  language: zh-CN
  timezone: Asia/Shanghai
llm:
  providers:
    anthropic:
      api_key: test-key
      default_model: claude-sonnet
    openai:
      api_key: test-key-2
      default_model: gpt-4o
  routing:
    strong: anthropic
    medium: anthropic
    light: openai
  cache:
    l1_enabled: true
memory:
  working_memory_limit: 100
  project_db_path: ./data/p.db
  user_db_path: ./data/u.db
  cleanup_interval_hours: 24
security:
  risk_threshold: L2
  allowed_paths: [~/projects]
profiles:
  default: developer
  available: [developer]
"#;
    tmp.write_all(yaml.as_bytes()).unwrap();
    let config = load_config(tmp.path()).unwrap();
    assert_eq!(config.llm.routing.strong, "anthropic");
    assert_eq!(config.llm.routing.light, "openai");
    assert_eq!(config.core.language, "zh-CN");
    assert_eq!(config.memory.working_memory_limit, 100);
}

#[test]
fn test_missing_config_file() {
    let result = load_config(std::path::Path::new("/nonexistent/eflow.yaml"));
    assert!(result.is_err());
}

#[test]
fn test_env_var_expansion_in_config() {
    unsafe {
        std::env::set_var("EFLOW_TEST_KEY", "expanded-key-value");
    }
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    let yaml = r#"
core:
  language: en
  timezone: UTC
llm:
  providers:
    anthropic:
      api_key: ${EFLOW_TEST_KEY}
      default_model: claude-sonnet
  routing:
    strong: anthropic
    medium: anthropic
    light: anthropic
  cache:
    l1_enabled: false
memory:
  working_memory_limit: 10
  project_db_path: ./p.db
  user_db_path: ./u.db
  cleanup_interval_hours: 1
security:
  risk_threshold: L0
  allowed_paths: []
profiles:
  default: developer
  available: [developer]
"#;
    tmp.write_all(yaml.as_bytes()).unwrap();
    let config = load_config(tmp.path()).unwrap();
    assert_eq!(
        config.llm.providers.anthropic.unwrap().api_key,
        "expanded-key-value"
    );
    unsafe {
        std::env::remove_var("EFLOW_TEST_KEY");
    }
}

#[test]
fn parses_llm_provider_timeout_and_retry() {
    // v1.1 Task A1: 扩 LlmConfig 加 timeout_secs / max_retries / retry_backoff_ms
    // + CacheConfig 加 l2_enabled / l2_ttl_days
    let yaml = r#"
core:
  language: zh-CN
  timezone: Asia/Shanghai
llm:
  providers:
    anthropic:
      api_key: sk-test
      default_model: claude-sonnet-4-6
      timeout_secs: 45
      max_retries: 5
      retry_backoff_ms: 500
  routing:
    strong: anthropic
    medium: anthropic
    light: anthropic
  cache:
    l1_enabled: true
    l2_enabled: true
    l2_ttl_days: 7
memory:
  working_memory_limit: 1000
  project_db_path: ./p.db
  user_db_path: ./u.db
  cleanup_interval_hours: 24
security:
  risk_threshold: L2
  allowed_paths: []
profiles:
  default: developer
  available: [developer]
"#;
    let cfg: eflow::infrastructure::config::EflowConfig =
        serde_yaml::from_str(yaml).expect("parse ok");
    let anthro = cfg.llm.providers.anthropic.as_ref().unwrap();
    assert_eq!(anthro.timeout_secs, 45);
    assert_eq!(anthro.max_retries, 5);
    assert_eq!(anthro.retry_backoff_ms, 500);
    assert!(cfg.llm.cache.l2_enabled);
    assert_eq!(cfg.llm.cache.l2_ttl_days, 7);
}
