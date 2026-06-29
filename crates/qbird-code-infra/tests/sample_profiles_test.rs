use qbird_code_infra::profile::Profile;
use tempfile::TempDir;

#[test]
fn test_default_profiles_created_on_first_run() {
    let dir = TempDir::new().unwrap();
    let profile_dir = dir.path().join("profiles");

    // First run: directory does not yet exist.
    Profile::create_sample_profiles(&profile_dir).expect("create_sample_profiles should succeed");

    // Both files must exist.
    assert!(
        profile_dir.join("developer.yaml").exists(),
        "developer.yaml should be created"
    );
    assert!(
        profile_dir.join("researcher.yaml").exists(),
        "researcher.yaml should be created"
    );

    // Validate content loads correctly.
    let dev = Profile::load(&profile_dir, "developer").expect("load developer");
    assert_eq!(dev.name, "developer");
    assert_eq!(
        dev.description.as_deref(),
        Some("Generic development assistant (multi-task)")
    );
    assert_eq!(dev.risk_threshold.as_deref(), Some("L3"));
    assert!(dev.tools_allow.is_empty(), "developer allows all tools");
    assert!(
        dev.system_prompt
            .as_deref()
            .unwrap_or("")
            .contains("先理解用户意图和约束"),
        "developer system_prompt 应包含 v0.3.1+ 中性多任务框架; got: {:?}",
        dev.system_prompt
    );

    let res = Profile::load(&profile_dir, "researcher").expect("load researcher");
    assert_eq!(res.name, "researcher");
    assert_eq!(
        res.description.as_deref(),
        Some("Read-only research assistant")
    );
    assert_eq!(res.risk_threshold.as_deref(), Some("L1"));
    assert_eq!(
        res.tools_allow,
        vec!["read_file", "search_code", "glob", "list_dir", "web_fetch"]
    );
    assert!(
        res.system_prompt
            .as_deref()
            .unwrap_or("")
            .contains("调研 profile"),
        "researcher system_prompt 应包含 v0.3.1+ 调研 profile 框架; got: {:?}",
        res.system_prompt
    );
}

#[test]
fn test_existing_profiles_not_overwritten() {
    let dir = TempDir::new().unwrap();
    let profile_dir = dir.path().join("profiles");
    std::fs::create_dir_all(&profile_dir).unwrap();

    // Pre-existing custom user profile (sample_version >= current) must be left alone.
    let custom_yaml = r#"name: custom
description: "My custom profile"
system_prompt: "Custom prompt"
sample_version: "0.3.1"
tools_allow: [read_file]
risk_threshold: L0
"#;
    std::fs::write(profile_dir.join("custom.yaml"), custom_yaml).unwrap();

    // Call creation logic — should NOT overwrite the custom profile.
    Profile::create_sample_profiles(&profile_dir).expect("should succeed without overwriting");

    // custom.yaml must still exist, untouched.
    let list = Profile::list(&profile_dir).expect("list");
    assert!(
        list.contains(&"custom".to_string()),
        "user's custom profile must not be removed; list = {:?}",
        list
    );

    // Verify the custom profile content is unchanged.
    let custom = Profile::load(&profile_dir, "custom").expect("load custom");
    assert_eq!(custom.name, "custom");
    assert_eq!(custom.description.as_deref(), Some("My custom profile"));
    assert_eq!(custom.system_prompt.as_deref(), Some("Custom prompt"));
    assert_eq!(custom.risk_threshold.as_deref(), Some("L0"));
}

#[test]
fn test_stale_sample_profile_refreshed() {
    let dir = TempDir::new().unwrap();
    let profile_dir = dir.path().join("profiles");
    std::fs::create_dir_all(&profile_dir).unwrap();

    // Pre-existing stale developer.yaml (no sample_version field → older than 0.3.1)
    let stale_yaml = r#"name: developer
description: "Rust development assistant"
system_prompt: "Old prompt"
tools_allow: []
risk_threshold: L3
"#;
    std::fs::write(profile_dir.join("developer.yaml"), stale_yaml).unwrap();

    Profile::create_sample_profiles(&profile_dir).expect("refresh should succeed");

    let dev = Profile::load(&profile_dir, "developer").expect("load developer");
    assert_eq!(
        dev.description.as_deref(),
        Some("Generic development assistant (multi-task)"),
        "stale developer.yaml must be refreshed to v0.3.1+ content"
    );
    assert!(
        dev.system_prompt
            .as_deref()
            .unwrap_or("")
            .contains("先理解用户意图和约束"),
        "system_prompt should be v0.3.1+ content; got: {:?}",
        dev.system_prompt
    );
}
