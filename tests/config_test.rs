rust_i18n::i18n!("locales", fallback = "en-US");

use qingbird_code::infrastructure::config::load_config;
use std::io::Write;

#[test]
fn test_load_valid_config() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    let yaml = r#"
core:
  language: zh-CN
  timezone: Asia/Shanghai
llm:
  deepseek:
    api_key: "sk-test"
    base_url: "https://api.deepseek.com"
    default_model: "deepseek-chat"
    timeout_secs: 30
    max_retries: 3
    retry_backoff_ms: 1000
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
    assert_eq!(config.core.language, "zh-CN");
    assert_eq!(config.memory.working_memory_limit, 100);
    assert!(config.llm.deepseek.default_model.as_deref() == Some("deepseek-chat"));
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
  deepseek:
    api_key: "${EFLOW_TEST_KEY}"
    base_url: "https://api.deepseek.com"
    default_model: "deepseek-chat"
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
    // deepseek.api_key 引用了 env var，应被 expand
    assert_eq!(
        config.llm.deepseek.api_key.as_deref(),
        Some("expanded-key-value")
    );
    unsafe {
        std::env::remove_var("EFLOW_TEST_KEY");
    }
}

#[test]
fn parses_llm_cache_defaults() {
    let yaml = r#"
core:
  language: zh-CN
  timezone: Asia/Shanghai
llm:
  deepseek:
    api_key: "sk-test"
    base_url: "https://api.deepseek.com"
    default_model: "deepseek-chat"
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
    let cfg: qingbird_code::infrastructure::config::EflowConfig =
        serde_yaml::from_str(yaml).expect("parse ok");
    assert!(cfg.llm.cache.l2_enabled);
    assert_eq!(cfg.llm.cache.l2_ttl_days, 7);
}
