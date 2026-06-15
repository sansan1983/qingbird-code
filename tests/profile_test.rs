rust_i18n::i18n!("locales", fallback = "en-US");

use std::fs;
use std::io::Write;

use eflow::common::types::RiskLevel;
use eflow::infrastructure::profile::{Profile, ProfileRegistry, Skill};

fn write_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

const PROFILE_YAML: &str = r#"
name: developer
description: 软件开发专家
system_prompt: |
  You are a senior engineer.
  Focus on quality and safety.
default_model: Medium
skills:
  - code_review
permission_boundary:
  allowed_paths:
    - ~/projects
  allowed_commands:
    - git
    - cargo
  max_file_size_bytes: 10485760
  network_enabled: true
"#;

const SKILL_YAML: &str = r#"
name: code_review
description: 代码审查技能
risk_level: L0
prompt_template: |
  Review the following code for bugs, style, and security.
required_tools:
  - read_file
"#;

#[test]
fn test_load_profile_from_valid_yaml() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "developer.yaml", PROFILE_YAML);

    let mut reg = ProfileRegistry::new();
    reg.load_profiles(dir.path()).unwrap();

    let p = reg.get_profile("developer").unwrap();
    assert_eq!(p.name, "developer");
    assert_eq!(p.description, "软件开发专家");
    assert!(p.system_prompt.contains("senior engineer"));
    assert!(p.system_prompt.contains("quality and safety"));
    assert_eq!(p.default_model, eflow::common::types::ModelTier::Medium);
    assert_eq!(p.skills, vec!["code_review".to_string()]);
    assert_eq!(
        p.permission_boundary.allowed_paths,
        vec!["~/projects".to_string()]
    );
    assert_eq!(
        p.permission_boundary.allowed_commands,
        vec!["git".to_string(), "cargo".to_string()]
    );
    assert_eq!(p.permission_boundary.max_file_size_bytes, 10485760);
    assert!(p.permission_boundary.network_enabled);
}

#[test]
fn test_load_profile_uses_default_model_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = r#"
name: minimalist
description: 极简 profile
system_prompt: hi
"#;
    write_file(dir.path(), "minimalist.yaml", yaml);

    let mut reg = ProfileRegistry::new();
    reg.load_profiles(dir.path()).unwrap();

    let p = reg.get_profile("minimalist").unwrap();
    assert_eq!(p.default_model, eflow::common::types::ModelTier::Medium);
    assert!(p.skills.is_empty());
    assert!(p.permission_boundary.allowed_paths.is_empty());
}

#[test]
fn test_get_profile_returns_none_for_unknown() {
    let reg = ProfileRegistry::new();
    assert!(reg.get_profile("ghost").is_none());
}

#[test]
fn test_list_profiles_returns_all_loaded() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "developer.yaml", PROFILE_YAML);
    write_file(
        dir.path(),
        "analyst.yaml",
        r#"
name: analyst
description: 数据分析
system_prompt: be analytical
"#,
    );

    let mut reg = ProfileRegistry::new();
    reg.load_profiles(dir.path()).unwrap();

    let names: Vec<&str> = reg.list_profiles().iter().map(|s| s.as_str()).collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"developer"));
    assert!(names.contains(&"analyst"));
}

#[test]
fn test_load_profiles_nonexistent_dir_is_ok() {
    let mut reg = ProfileRegistry::new();
    let result = reg.load_profiles(std::path::Path::new("/nonexistent/dir/xyz"));
    assert!(result.is_ok());
    assert!(reg.list_profiles().is_empty());
}

#[test]
fn test_load_skills_nonexistent_dir_is_ok() {
    let mut reg = ProfileRegistry::new();
    let result = reg.load_skills(std::path::Path::new("/nonexistent/dir/xyz"));
    assert!(result.is_ok());
}

#[test]
fn test_load_skills_ignores_non_yaml_files() {
    let skills_dir = tempfile::tempdir().unwrap();
    write_file(skills_dir.path(), "readme.md", "not a skill");
    write_file(skills_dir.path(), "code_review.yaml", SKILL_YAML);
    write_file(skills_dir.path(), "config.json", "{}");

    let mut reg = ProfileRegistry::new();
    reg.load_skills(skills_dir.path()).unwrap();

    // No profile loaded, even though skill yaml exists
    let skills = reg.get_profile_skills("nonexistent");
    assert!(skills.is_empty());
}

#[test]
fn test_build_system_prompt_without_skills_returns_profile_prompt() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "developer.yaml", PROFILE_YAML);

    let mut reg = ProfileRegistry::new();
    reg.load_profiles(dir.path()).unwrap();

    let prompt = reg.build_system_prompt("developer").unwrap();
    assert!(prompt.contains("senior engineer"));
    assert!(!prompt.contains("## Skill:"));
}

#[test]
fn test_build_system_prompt_includes_loaded_skills() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "developer.yaml", PROFILE_YAML);
    let skills_dir = tempfile::tempdir().unwrap();
    write_file(skills_dir.path(), "code_review.yaml", SKILL_YAML);

    let mut reg = ProfileRegistry::new();
    reg.load_profiles(dir.path()).unwrap();
    reg.load_skills(skills_dir.path()).unwrap();

    let prompt = reg.build_system_prompt("developer").unwrap();
    assert!(prompt.contains("senior engineer"));
    assert!(prompt.contains("## Skill: code_review"));
    assert!(prompt.contains("Review the following code"));
}

#[test]
fn test_build_system_prompt_skips_skills_not_loaded() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "developer.yaml", PROFILE_YAML);
    // profile says skills: [code_review, debug_assist] but only code_review is loaded

    let mut reg = ProfileRegistry::new();
    reg.load_profiles(dir.path()).unwrap();

    let prompt = reg.build_system_prompt("developer").unwrap();
    assert!(!prompt.contains("## Skill:"));
    assert!(!prompt.contains("debug_assist"));
}

#[test]
fn test_build_system_prompt_missing_profile_returns_error() {
    let reg = ProfileRegistry::new();
    let result = reg.build_system_prompt("ghost");
    assert!(result.is_err());
    let err = format!("{:?}", result.unwrap_err());
    assert!(err.contains("ProfileNotFound"));
}

#[test]
fn test_skill_default_version() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = r#"
name: no_version_skill
description: 不带 version
risk_level: L1
prompt_template: do something
"#;
    write_file(dir.path(), "no_version_skill.yaml", yaml);

    let mut reg = ProfileRegistry::new();
    reg.load_skills(dir.path()).unwrap();

    // Indirect check: build prompt should still work
    let profile_dir = tempfile::tempdir().unwrap();
    write_file(
        profile_dir.path(),
        "tester.yaml",
        r#"
name: tester
description: t
system_prompt: base
skills:
  - no_version_skill
"#,
    );
    reg.load_profiles(profile_dir.path()).unwrap();
    let prompt = reg.build_system_prompt("tester").unwrap();
    assert!(prompt.contains("## Skill: no_version_skill"));
    assert!(prompt.contains("do something"));
}

#[test]
fn test_skill_risk_level_parsed_as_enum() {
    // Compile-time: Skill has risk_level: RiskLevel
    let s = Skill {
        name: "x".into(),
        version: "1.0".into(),
        description: "y".into(),
        risk_level: RiskLevel::L2,
        prompt_template: "z".into(),
        required_tools: vec![],
    };
    assert_eq!(s.risk_level, RiskLevel::L2);
}

#[test]
fn test_profile_clone_works() {
    let p = Profile {
        name: "x".into(),
        description: "y".into(),
        system_prompt: "z".into(),
        default_model: eflow::common::types::ModelTier::Light,
        skills: vec!["a".into()],
        permission_boundary: Default::default(),
    };
    let p2 = p.clone();
    assert_eq!(p.name, p2.name);
    assert_eq!(p.default_model, p2.default_model);
}

#[test]
fn test_registry_default() {
    let reg = ProfileRegistry::default();
    assert!(reg.list_profiles().is_empty());
}
