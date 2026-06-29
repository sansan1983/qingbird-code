//! Integration test for `--profile` flag behavior.
//!
//! Brief allows a function-level fallback because the full binary invocation
//! is fragile in CI. We exercise the same code path the CLI takes:
//! `Profile::load → merge_into` and assert that the merged system prompt
//! (or any of the override fields) reflects the profile's data correctly.

use std::fs;

use qbird_code_infra::profile::Profile;
use tempfile::TempDir;

fn write_yaml(dir: &std::path::Path, name: &str, body: &str) {
    let p = dir.join(format!("{name}.yaml"));
    fs::write(&p, body).expect("write profile yaml");
}

#[test]
fn test_cli_profile_flag_overrides_yaml() {
    // Emulate qingbird.yaml `profiles: { default: "" }` (no default) + a profile
    // file at the conventional location. Then exercise the same merge flow the
    // CLI's `--profile <name>` flag uses.
    let dir = TempDir::new().unwrap();
    write_yaml(
        dir.path(),
        "developer",
        r#"
name: developer
system_prompt: "PROFILE_OVERRIDE: You are a senior Rust developer."
tools_allow:
  - read_file
  - write_file
risk_threshold: L1
"#,
    );

    // CLI: --profile developer
    let name = "developer";
    let p = Profile::load(dir.path(), name).expect("load");

    // === merge path (mirrors main.rs startup) ===
    let mut sp = String::from("DEFAULT: You are an AI coding assistant.");
    let mut allowed: Option<Vec<String>> = None;
    let mut risk: Option<String> = None;
    let mut provider_active = String::from("deepseek");
    let mut model = String::from("deepseek-v4-pro");
    let mut warnings: Vec<String> = Vec::new();

    p.merge_into(
        &mut sp,
        &mut allowed,
        &mut risk,
        &mut provider_active,
        &mut model,
        &mut warnings,
    );
    // No provider/model override in this fixture → no warnings.
    assert!(
        warnings.is_empty(),
        "no provider/model fields → no warnings (got {warnings:?})"
    );

    // Profile's system_prompt must REPLACE the default.
    assert!(
        sp.starts_with("PROFILE_OVERRIDE"),
        "profile.system_prompt must replace default, got: {sp}"
    );

    // tools_allow goes into `allowed` (Some(list)).
    let allow = allowed.expect("profile.tools_allow was set, allowed should be Some");
    assert_eq!(
        allow,
        vec!["read_file".to_string(), "write_file".to_string()]
    );

    // risk_threshold converted to the profile's setting.
    assert_eq!(risk.as_deref(), Some("L1"));
}

#[test]
fn test_cli_profile_flag_missing_profile_exits_path() {
    // Mirror the error path: bad profile name -> ProfileNotFound user_message.
    let dir = TempDir::new().unwrap();
    let res = Profile::load(dir.path(), "no-such-profile");
    assert!(res.is_err(), "missing profile must error");
    let msg = res.unwrap_err().user_message();
    assert!(
        msg.contains("no-such-profile"),
        "user_message must echo profile name, got: {msg}"
    );
}

#[test]
fn test_cli_profile_flag_list_includes_user_profiles() {
    // Mirror `/profile list` in REPL: enumerate profile names in profile_dir.
    let dir = TempDir::new().unwrap();
    write_yaml(dir.path(), "developer", "name: developer\n");
    write_yaml(dir.path(), "researcher", "name: researcher\n");
    let list = Profile::list(dir.path()).expect("list");
    assert_eq!(
        list,
        vec!["developer".to_string(), "researcher".to_string()]
    );
}
