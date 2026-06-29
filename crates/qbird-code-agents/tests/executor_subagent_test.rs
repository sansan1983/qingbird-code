use std::collections::HashMap;
use std::sync::Arc;

use qbird_code_agents::react_loop::ReactLoopConfig;
use qbird_code_agents::subagent::executor::SubagentExecutor;
use qbird_code_agents::subagent::profile::{SubagentMode, SubagentProfile, ToolPolicy};
use qbird_code_models::EflowError;
use qbird_code_tools::ToolRegistry;

fn make_profiles() -> HashMap<String, SubagentProfile> {
    let mut m = HashMap::new();
    m.insert(
        "explore".into(),
        SubagentProfile {
            name: "explore".into(),
            mode: SubagentMode::Subagent,
            tool_policy: ToolPolicy::ReadOnly,
            prompt_preamble: "你是探索代理".into(),
            description: "test".into(),
            default_tools: vec!["read_file".into(), "search_code".into()],
            max_iterations: Some(5),
            model: None,
        },
    );
    m
}

#[test]
fn executor_builder_succeeds_with_valid_inputs() {
    let profiles = make_profiles();
    let registry = Arc::new(ToolRegistry::new());
    let executor = SubagentExecutor::builder()
        .profiles(profiles)
        .base_config(ReactLoopConfig::default())
        .tool_registry(registry)
        .build();
    assert!(executor.is_ok());
}

#[test]
fn executor_validate_profile_returns_not_found_for_unknown() {
    let profiles = make_profiles();
    let registry = Arc::new(ToolRegistry::new());
    let executor = SubagentExecutor::builder()
        .profiles(profiles)
        .base_config(ReactLoopConfig::default())
        .tool_registry(registry)
        .build()
        .unwrap();
    let result = executor.validate_profile("nonexistent");
    assert!(matches!(
        result,
        Err(EflowError::SubagentProfileNotFound { .. })
    ));
}

#[test]
fn executor_list_profile_names_returns_all_loaded() {
    let profiles = make_profiles();
    let registry = Arc::new(ToolRegistry::new());
    let executor = SubagentExecutor::builder()
        .profiles(profiles)
        .base_config(ReactLoopConfig::default())
        .tool_registry(registry)
        .build()
        .unwrap();
    let names = executor.list_profile_names();
    assert!(names.contains(&"explore".to_string()));
}

#[test]
fn executor_builder_missing_profiles_errors() {
    let registry = Arc::new(ToolRegistry::new());
    let result = SubagentExecutor::builder()
        .base_config(ReactLoopConfig::default())
        .tool_registry(registry)
        .build();
    assert!(result.is_err());
}
