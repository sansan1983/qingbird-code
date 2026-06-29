use std::collections::HashMap;

use qbird_code_agents::subagent::config::{
    ProfileOverride, load_profiles_from_yaml, merge_into_builtins,
};
use qbird_code_agents::subagent::profile::{SubagentProfile, ToolPolicy, builtin_profiles};

#[test]
fn load_profiles_from_yaml_empty_returns_builtins() {
    let map = load_profiles_from_yaml(None).expect("no yaml");
    let builtins = builtin_profiles();
    assert_eq!(map.len(), builtins.len());
    for b in builtins {
        assert!(map.contains_key(&b.name), "missing builtin {}", b.name);
    }
}

#[test]
fn load_profiles_from_yaml_user_override_replaces_field() {
    let yaml = r#"
profiles:
  general:
    prompt_preamble: "覆盖后的提示词"
    max_iterations: 30
"#;
    let map = load_profiles_from_yaml(Some(yaml)).expect("parse yaml");
    let general = map.get("general").expect("general exists");
    assert_eq!(general.prompt_preamble, "覆盖后的提示词");
    assert_eq!(general.max_iterations, Some(30));
    assert_eq!(general.tool_policy, ToolPolicy::Inherit);
}

#[test]
fn load_profiles_from_yaml_user_adds_new_profile() {
    let yaml = r#"
profiles:
  my-custom:
    prompt_preamble: "自定义代理"
    tool_policy: readonly
    description: "测试"
"#;
    let map = load_profiles_from_yaml(Some(yaml)).expect("parse yaml");
    let custom = map.get("my-custom").expect("my-custom exists");
    assert_eq!(custom.prompt_preamble, "自定义代理");
    assert_eq!(custom.tool_policy, ToolPolicy::ReadOnly);
    assert!(map.contains_key("general"));
    assert!(map.contains_key("explore"));
}

#[test]
fn load_profiles_malformed_yaml_returns_error() {
    let yaml = "this is: not: valid: yaml: [[[";
    let result = load_profiles_from_yaml(Some(yaml));
    assert!(result.is_err());
}

#[test]
fn merge_into_builtins_user_wins_per_field() {
    let mut map: HashMap<String, SubagentProfile> = builtin_profiles()
        .into_iter()
        .map(|p| (p.name.clone(), p))
        .collect();
    let user_config = (
        "general".to_string(),
        ProfileOverride {
            mode: None,
            tool_policy: Some(ToolPolicy::ReadOnly),
            prompt_preamble: Some("新提示".into()),
            description: None,
            default_tools: None,
            max_iterations: Some(20),
            model: None,
        },
    );
    merge_into_builtins(&mut map, &[user_config]);
    let g = map.get("general").unwrap();
    assert_eq!(g.tool_policy, ToolPolicy::ReadOnly);
    assert_eq!(g.prompt_preamble, "新提示");
    assert_eq!(g.max_iterations, Some(20));
    assert!(g.description.contains("通用代理"));
}
