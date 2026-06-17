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
    // v1.3: env var expansion 在 expand_env_vars 阶段处理整段 yaml
    // 这里验证 eflow.yaml 内任意字段（含 routing）都做 expansion。
    unsafe {
        std::env::set_var("EFLOW_TEST_KEY", "expanded-key-value");
    }
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    let yaml = r#"
core:
  language: en
  timezone: UTC
llm:
  routing:
    strong: ${EFLOW_TEST_KEY}
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
    // v1.3: routing.strong 引用了 env var，应被 expand
    assert_eq!(config.llm.routing.strong, "expanded-key-value");
    unsafe {
        std::env::remove_var("EFLOW_TEST_KEY");
    }
}

#[test]
fn parses_llm_provider_timeout_and_retry() {
    // v1.3: timeout / retry 已迁到 ProviderConfig（per-provider YAML 字段）
    // 这里只测 cache 字段仍在 LlmConfig 里。
    let yaml = r#"
core:
  language: zh-CN
  timezone: Asia/Shanghai
llm:
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
    assert!(cfg.llm.cache.l2_enabled);
    assert_eq!(cfg.llm.cache.l2_ttl_days, 7);
}
