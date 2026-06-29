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
        Some("Rust development assistant")
    );
    assert_eq!(dev.risk_threshold.as_deref(), Some("L3"));
    assert!(dev.tools_allow.is_empty(), "developer allows all tools");
    assert_eq!(
        dev.system_prompt.as_deref(),
        Some(
            "你是一个 Rust 开发助手，专注于帮助用户编写、审查和改进 Rust 代码。\n\n工作方式：\n- 先理解用户的具体需求和约束，再给出方案\n- 尊重现有代码风格和约定，不做大范围重写\n- 读当前状态再行动；不确定时询问或检查文件\n- 使用中文回复，代码注释保持英文\n"
        )
    );

    let res = Profile::load(&profile_dir, "researcher").expect("load researcher");
    assert_eq!(res.name, "researcher");
    assert_eq!(
        res.description.as_deref(),
        Some("Research assistant (read-only)")
    );
    assert_eq!(res.risk_threshold.as_deref(), Some("L1"));
    assert_eq!(
        res.tools_allow,
        vec!["read_file", "search_code", "glob", "list_dir", "web_fetch"]
    );
    assert_eq!(
        res.system_prompt.as_deref(),
        Some(
            "你是一个研究助手，专注于信息检索、整合与分析。\n只使用只读工具收集信息，不修改任何文件；找到答案后清晰汇报发现。\n"
        )
    );
}

#[test]
fn test_existing_profiles_not_overwritten() {
    let dir = TempDir::new().unwrap();
    let profile_dir = dir.path().join("profiles");
    std::fs::create_dir_all(&profile_dir).unwrap();

    // Pre-existing custom profile.
    let custom_yaml = r#"name: custom
description: "My custom profile"
system_prompt: "Custom prompt"
tools_allow: [read_file]
risk_threshold: L0
"#;
    std::fs::write(profile_dir.join("custom.yaml"), custom_yaml).unwrap();

    // Call creation logic — should NOT overwrite or add files.
    Profile::create_sample_profiles(&profile_dir).expect("should succeed without overwriting");

    let list = Profile::list(&profile_dir).expect("list");
    assert_eq!(
        list,
        vec!["custom"],
        "existing profiles must not be modified"
    );

    // Verify the custom profile content is unchanged.
    let custom = Profile::load(&profile_dir, "custom").expect("load custom");
    assert_eq!(custom.name, "custom");
    assert_eq!(custom.description.as_deref(), Some("My custom profile"));
    assert_eq!(custom.system_prompt.as_deref(), Some("Custom prompt"));
    assert_eq!(custom.risk_threshold.as_deref(), Some("L0"));
}
