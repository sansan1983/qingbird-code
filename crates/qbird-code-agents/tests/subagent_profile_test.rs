use qbird_code_agents::subagent::profile::{
    SubagentMode, SubagentProfile, ToolPolicy, builtin_profiles,
};

#[test]
fn builtin_profiles_returns_5_entries() {
    assert_eq!(builtin_profiles().len(), 5);
}

#[test]
fn builtin_profiles_have_unique_names() {
    let profiles = builtin_profiles();
    let mut names: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 5);
}

#[test]
fn builtin_general_uses_inherit_policy() {
    let profiles = builtin_profiles();
    let general = profiles
        .into_iter()
        .find(|p| p.name == "general")
        .expect("general must exist");
    assert_eq!(general.tool_policy, ToolPolicy::Inherit);
    assert_eq!(general.mode, SubagentMode::Subagent);
    assert!(general.model.is_none());
}

#[test]
fn builtin_explore_uses_readonly_with_default_tools() {
    let profiles = builtin_profiles();
    let explore = profiles
        .into_iter()
        .find(|p| p.name == "explore")
        .expect("explore must exist");
    assert_eq!(explore.tool_policy, ToolPolicy::ReadOnly);
    assert!(explore.default_tools.contains(&"read_file".to_string()));
    assert!(explore.max_iterations.is_some());
}

#[test]
fn builtin_planner_and_reviewer_are_readonly() {
    let profiles = builtin_profiles();
    for name in ["planner", "reviewer"] {
        let p = profiles
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{} must exist", name));
        assert_eq!(
            p.tool_policy,
            ToolPolicy::ReadOnly,
            "{} should be read-only",
            name
        );
    }
}

#[test]
fn read_only_tool_names_contains_expected_tools() {
    let names = SubagentProfile::read_only_tool_names();
    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"search_code"));
    assert!(names.contains(&"glob"));
    assert!(names.contains(&"list_dir"));
    assert!(names.contains(&"web_fetch"));
}
