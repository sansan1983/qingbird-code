use qbird_code_infra::profile::Profile;
use tempfile::TempDir;

// ===== Test helpers =====

fn write_profile(dir: &std::path::Path, name: &str, body: &str) {
    let path = dir.join(format!("{name}.yaml"));
    std::fs::write(&path, body).expect("write profile");
}

// ===== Acceptance tests =====

#[test]
fn test_load_valid_profile() {
    let dir = TempDir::new().unwrap();
    write_profile(
        dir.path(),
        "developer",
        r#"
name: developer
description: "Developer-mode profile"
system_prompt: "You are a senior Rust developer."
tools_allow:
  - read_file
  - write_file
  - execute_command
risk_threshold: L1
provider: deepseek
model: deepseek-v4-pro
"#,
    );
    let p = Profile::load(dir.path(), "developer").expect("load");
    assert_eq!(p.name, "developer");
    assert_eq!(
        p.system_prompt.as_deref(),
        Some("You are a senior Rust developer.")
    );
    assert_eq!(
        p.tools_allow,
        vec!["read_file", "write_file", "execute_command"]
    );
    assert_eq!(p.risk_threshold.as_deref(), Some("L1"));
    assert_eq!(p.provider.as_deref(), Some("deepseek"));
    assert_eq!(p.model.as_deref(), Some("deepseek-v4-pro"));
    assert_eq!(p.description.as_deref(), Some("Developer-mode profile"));
}

#[test]
fn test_load_missing_file_errors() {
    let dir = TempDir::new().unwrap();
    let res = Profile::load(dir.path(), "nonexistent");
    assert!(res.is_err(), "expected error for missing profile");
}

#[test]
fn test_load_malformed_yaml_errors() {
    let dir = TempDir::new().unwrap();
    write_profile(
        dir.path(),
        "bad",
        "name: bad\n  broken_indent: : :\n[not yaml",
    );
    let res = Profile::load(dir.path(), "bad");
    assert!(res.is_err(), "expected error for malformed yaml");
}

#[test]
fn test_list_returns_sorted() {
    let dir = TempDir::new().unwrap();
    write_profile(dir.path(), "zeta", "name: zeta\n");
    write_profile(dir.path(), "alpha", "name: alpha\n");
    write_profile(dir.path(), "beta", "name: beta\n");
    let list = Profile::list(dir.path()).expect("list");
    assert_eq!(list, vec!["alpha", "beta", "zeta"]);
}

#[test]
fn test_merge_into_applies_all_sections() {
    let p = Profile {
        name: "researcher".into(),
        system_prompt: Some("You are a research assistant.".into()),
        tools_allow: vec!["read_file".into(), "web_fetch".into()],
        risk_threshold: Some("L0".into()),
        provider: Some("openai".into()),
        model: Some("gpt-4o".into()),
        description: None,
    };
    let mut sp = String::new();
    let mut allowed: Option<Vec<String>> = None;
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
    assert_eq!(sp, "You are a research assistant.");
    assert_eq!(
        allowed,
        Some(vec!["read_file".to_string(), "web_fetch".to_string()])
    );
    assert_eq!(risk.as_deref(), Some("L0"));
    assert_eq!(provider, "openai");
    assert_eq!(model, "gpt-4o");
    // Both provider and model differ from defaults → two warnings.
    assert_eq!(
        warnings.len(),
        2,
        "expected 2 warnings for provider+model override"
    );
    assert!(warnings[0].contains("provider"));
    assert!(warnings[1].contains("model"));
}

#[test]
fn test_merge_into_partial_profile() {
    // Only system_prompt is set; everything else must remain unchanged.
    let p = Profile {
        name: "minimal".into(),
        system_prompt: Some("Minimal prompt.".into()),
        tools_allow: vec![],
        risk_threshold: None,
        provider: None,
        model: None,
        description: None,
    };
    let mut sp = String::from("old prompt");
    let mut allowed: Option<Vec<String>> = Some(vec!["old".into()]);
    let mut risk = Some("L1".to_string());
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
    assert_eq!(sp, "Minimal prompt.");
    // tools_allow empty → allowed_tools stay at whatever was set
    assert_eq!(allowed, Some(vec!["old".to_string()]));
    assert_eq!(risk.as_deref(), Some("L1"));
    assert_eq!(provider, "deepseek");
    assert_eq!(model, "deepseek-v4-pro");
    // No provider/model override → no warnings.
    assert!(
        warnings.is_empty(),
        "expected no warnings when provider/model inherit"
    );
}

#[test]
fn test_cli_profile_overrides_yaml() {
    // Simulate the --profile flag path: load cfg.profiles.default from yaml
    // (here emulated by a profile file in a custom dir), then load that
    // named profile and merge — the profile's system_prompt wins.
    let dir = TempDir::new().unwrap();
    write_profile(
        dir.path(),
        "developer",
        "name: developer\nsystem_prompt: \"from-profile\"\n",
    );

    // The "yaml default" is "developer" (matches qingbird.yaml profiles.default).
    let yaml_default = "developer".to_string();
    let chosen = Profile::load(dir.path(), &yaml_default).expect("load");
    let mut sp = String::from("from-yaml-system-prompt");
    let mut allowed: Option<Vec<String>> = None;
    let mut risk: Option<String> = None;
    let mut provider = String::from("deepseek");
    let mut model = String::from("deepseek-v4-pro");
    let mut warnings: Vec<String> = Vec::new();
    chosen.merge_into(
        &mut sp,
        &mut allowed,
        &mut risk,
        &mut provider,
        &mut model,
        &mut warnings,
    );
    assert_eq!(sp, "from-profile", "profile must override yaml default");
    assert!(
        warnings.is_empty(),
        "no provider/model in this profile → no warnings"
    );
}

#[test]
fn test_slash_profile_switch() {
    // Simulates the /profile <name> path: load + merge_into with current state.
    let dir = TempDir::new().unwrap();
    write_profile(
        dir.path(),
        "researcher",
        r#"
name: researcher
system_prompt: "Switched to researcher mode."
tools_allow: [read_file, search_code, web_fetch]
risk_threshold: L0
"#,
    );
    // Current state (pre-switch).
    let mut sp = String::from("developer-mode prompt");
    let mut allowed: Option<Vec<String>> = Some(vec![
        "read_file".into(),
        "write_file".into(),
        "execute_command".into(),
    ]);
    let mut risk = Some("L2".to_string());
    let mut provider = String::from("deepseek");
    let mut model = String::from("deepseek-v4-pro");
    let mut warnings: Vec<String> = Vec::new();
    // Load + apply
    let p = Profile::load(dir.path(), "researcher").expect("load");
    p.merge_into(
        &mut sp,
        &mut allowed,
        &mut risk,
        &mut provider,
        &mut model,
        &mut warnings,
    );
    assert_eq!(sp, "Switched to researcher mode.");
    assert_eq!(
        allowed,
        Some(vec![
            "read_file".to_string(),
            "search_code".to_string(),
            "web_fetch".to_string()
        ])
    );
    assert_eq!(risk.as_deref(), Some("L0"));
    assert!(
        warnings.is_empty(),
        "this profile has no provider/model fields"
    );
}

#[test]
fn test_default_dir_is_data_dir_profiles() {
    let dir = Profile::default_dir();
    // On Windows, dirs::data_dir() typically yields AppData/Roaming; on Unix
    // $HOME/.local/share. We don't assert the OS-specific root — only that
    // it terminates with `qingbird/profiles`.
    let s = dir.to_string_lossy();
    assert!(
        s.ends_with("qingbird/profiles") || s.ends_with("qingbird\\profiles"),
        "default_dir must end with qingbird/profiles, got: {s}"
    );
}

#[test]
fn test_system_prompt_replaces_not_appends() {
    // Brief: profile.system_prompt REPLACES the current system prompt, does NOT append.
    let p = Profile {
        name: "replacer".into(),
        system_prompt: Some("new prompt".into()),
        tools_allow: vec![],
        risk_threshold: None,
        provider: None,
        model: None,
        description: None,
    };
    let mut sp = String::from("old prompt");
    let mut allowed: Option<Vec<String>> = None;
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
        sp, "new prompt",
        "system_prompt must REPLACE, not append (got {sp:?})"
    );
    assert!(warnings.is_empty(), "no provider/model → no warnings");
}

#[test]
fn test_merge_into_provider_override_emits_warning() {
    // Spec gap: profile.provider is merged into provider_active but the
    // live LLM (HttpLlmClient + Box<dyn Provider>) was constructed at
    // startup BEFORE the profile was applied. Caller should warn the user.
    let p = Profile {
        name: "switcher".into(),
        system_prompt: None,
        tools_allow: vec![],
        risk_threshold: None,
        provider: Some("ollama".into()),
        model: None,
        description: None,
    };
    let mut sp = String::new();
    let mut allowed: Option<Vec<String>> = None;
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
    assert_eq!(provider, "ollama");
    assert_eq!(warnings.len(), 1);
    assert!(
        warnings[0].contains("ollama"),
        "warning must name the provider value, got: {warnings:?}"
    );
    // Either locale may render — assert the substring "provider" or "重启"
    // both indicate the i18n template fired.
    let lower = warnings[0].to_lowercase();
    assert!(
        lower.contains("provider") || warnings[0].contains("provider"),
        "warning should reference the provider field, got: {warnings:?}"
    );
}

#[test]
fn test_merge_into_model_override_emits_warning() {
    let p = Profile {
        name: "switcher".into(),
        system_prompt: None,
        tools_allow: vec![],
        risk_threshold: None,
        provider: None,
        model: Some("llama3".into()),
        description: None,
    };
    let mut sp = String::new();
    let mut allowed: Option<Vec<String>> = None;
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
    assert_eq!(model, "llama3");
    assert_eq!(warnings.len(), 1);
    assert!(
        warnings[0].contains("llama3"),
        "warning must name the model value, got: {warnings:?}"
    );
    let lower = warnings[0].to_lowercase();
    assert!(
        lower.contains("model") || warnings[0].contains("model"),
        "warning should reference the model field, got: {warnings:?}"
    );
}

#[test]
fn test_merge_into_same_provider_no_warning() {
    // If the profile specifies the same provider that's already active,
    // no warning is needed — no override is happening.
    let p = Profile {
        name: "noop".into(),
        system_prompt: None,
        tools_allow: vec![],
        risk_threshold: None,
        provider: Some("deepseek".into()),
        model: Some("deepseek-v4-pro".into()),
        description: None,
    };
    let mut sp = String::new();
    let mut allowed: Option<Vec<String>> = None;
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
    assert!(
        warnings.is_empty(),
        "no override → no warning (got {warnings:?})"
    );
}
