use qbird_code_infra::profile::Profile;
use tempfile::TempDir;

fn write_profile(dir: &std::path::Path, name: &str, body: &str) {
    let path = dir.join(format!("{name}.yaml"));
    std::fs::write(&path, body).expect("write profile");
}

#[test]
fn test_list_empty_dir_returns_empty() {
    let dir = TempDir::new().unwrap();
    let empty = dir.path().join("empty_profiles");
    std::fs::create_dir_all(&empty).unwrap();
    let list = Profile::list(&empty).expect("list");
    assert!(list.is_empty());
}

#[test]
fn test_list_nonexistent_dir_returns_empty() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("no_such_dir");
    let list = Profile::list(&missing).expect("list");
    assert!(list.is_empty(), "non-existent dir should return empty vec");
}

#[test]
fn test_load_profile_with_empty_system_prompt() {
    let dir = TempDir::new().unwrap();
    write_profile(dir.path(), "minimal", "name: minimal\n");
    let p = Profile::load(dir.path(), "minimal").expect("load");
    assert!(p.system_prompt.is_none());
    assert!(p.tools_allow.is_empty());
    assert!(p.risk_threshold.is_none());
    assert!(p.provider.is_none());
    assert!(p.model.is_none());
}

#[test]
fn test_merge_into_empty_tools_allow_does_not_override() {
    let p = Profile {
        name: "no-tools".into(),
        system_prompt: None,
        tools_allow: vec![],
        risk_threshold: None,
        provider: None,
        model: None,
        description: None,
    };
    let mut sp = String::new();
    let mut allowed: Option<Vec<String>> = Some(vec!["read_file".into(), "write_file".into()]);
    let mut risk: Option<String> = None;
    let mut provider = String::from("deepseek");
    let mut model = String::from("deepseek-v4-pro");
    let mut warnings: Vec<String> = Vec::new();
    p.merge_into(
        &mut sp,
        &mut allowed,
        &mut risk,
        &mut provider,
        &mut model,
        &mut warnings,
    );
    assert_eq!(
        allowed,
        Some(vec!["read_file".to_string(), "write_file".to_string()]),
        "empty tools_allow should not override existing whitelist"
    );
}

#[test]
fn test_merge_into_overrides_all_fields_simultaneously() {
    let p = Profile {
        name: "full".into(),
        system_prompt: Some("new prompt".into()),
        tools_allow: vec!["tool_a".into()],
        risk_threshold: Some("L2".into()),
        provider: Some("ollama".into()),
        model: Some("llama3".into()),
        description: None,
    };
    let mut sp = String::from("old");
    let mut allowed: Option<Vec<String>> = None;
    let mut risk: Option<String> = Some("L3".into());
    let mut provider = String::from("deepseek");
    let mut model = String::from("deepseek-v4-pro");
    let mut warnings: Vec<String> = Vec::new();
    p.merge_into(
        &mut sp,
        &mut allowed,
        &mut risk,
        &mut provider,
        &mut model,
        &mut warnings,
    );
    assert_eq!(sp, "new prompt");
    assert_eq!(allowed, Some(vec!["tool_a".to_string()]));
    assert_eq!(risk.as_deref(), Some("L2"));
    assert_eq!(provider, "ollama");
    assert_eq!(model, "llama3");
    assert_eq!(warnings.len(), 2, "both provider and model changed");
}

#[test]
fn test_profile_load_name_mismatch_overrides() {
    let dir = TempDir::new().unwrap();
    // Profile yaml says name: alpha but we load as "beta" — load should override
    write_profile(dir.path(), "beta", "name: alpha\ndescription: test\n");
    let p = Profile::load(dir.path(), "beta").expect("load");
    assert_eq!(p.name, "beta", "load should inject the requested name");
}

#[test]
fn test_create_sample_profiles_idempotent() {
    let dir = TempDir::new().unwrap();
    let profile_dir = dir.path().join("profiles");
    Profile::create_sample_profiles(&profile_dir).expect("first run");
    Profile::create_sample_profiles(&profile_dir).expect("second run should be idempotent");
    let list = Profile::list(&profile_dir).expect("list");
    assert_eq!(list, vec!["developer", "researcher"]);
}
