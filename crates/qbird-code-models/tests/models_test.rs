use std::collections::HashSet;
use std::path::PathBuf;

use qbird_code_models::{
    Capability, Importance, MemoryCategory, Message, MessageRole, PermissionSet, RetryPolicy,
    RiskLevel, Role, ToolCall, ToolCallFunction, UsageStats,
};

// ===== Message =====

#[test]
fn test_message_system() {
    let msg = Message::system("Be helpful");
    assert_eq!(msg.role, MessageRole::System);
    assert_eq!(msg.content, "Be helpful");
    assert_eq!(msg.role_str(), "system");
    assert!(!msg.has_tool_calls());
}

#[test]
fn test_message_user() {
    let msg = Message::user("Hello");
    assert_eq!(msg.role, MessageRole::User);
    assert_eq!(msg.content, "Hello");
    assert_eq!(msg.role_str(), "user");
}

#[test]
fn test_message_assistant() {
    let msg = Message::assistant("Answer", Some("Thinking...".into()));
    assert_eq!(msg.role, MessageRole::Assistant);
    assert_eq!(msg.content, "Answer");
    assert_eq!(msg.reasoning_content, Some("Thinking...".into()));
}

#[test]
fn test_message_assistant_with_tools() {
    let tc = ToolCall {
        id: "call_1".into(),
        r#type: "function".into(),
        function: ToolCallFunction {
            name: "read_file".into(),
            arguments: r#"{"path": "foo.txt"}"#.into(),
        },
    };
    let msg = Message::assistant_with_tools("", None, vec![tc]);
    assert_eq!(msg.role, MessageRole::Assistant);
    assert!(msg.has_tool_calls());
}

#[test]
fn test_message_tool_result() {
    let msg = Message::tool_result("call_1".into(), "read_file".into(), "file content");
    assert_eq!(msg.role, MessageRole::Tool);
    assert_eq!(msg.tool_call_id, Some("call_1".into()));
    assert_eq!(msg.name, Some("read_file".into()));
}

#[test]
fn test_message_serde_roundtrip() {
    let msg = Message::assistant("Hi", Some("thinking...".into()));
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(msg.content, decoded.content);
    assert_eq!(msg.role, decoded.role);
    assert_eq!(msg.reasoning_content, decoded.reasoning_content);
}

// ===== RiskLevel =====

#[test]
fn test_risk_level_ordering() {
    assert!(RiskLevel::L0 < RiskLevel::L1);
    assert!(RiskLevel::L1 < RiskLevel::L2);
    assert!(RiskLevel::L2 < RiskLevel::L3);
    assert_eq!(RiskLevel::default(), RiskLevel::L0);
}

// ===== RetryPolicy (round-trip + backoff math) =====

#[test]
fn test_retry_policy_roundtrip() {
    let policy = RetryPolicy::default();
    let json = serde_json::to_string(&policy).unwrap();
    let decoded: RetryPolicy = serde_json::from_str(&json).unwrap();
    assert_eq!(policy.max_retries, decoded.max_retries);
    assert_eq!(policy.initial_backoff_ms, decoded.initial_backoff_ms);
    assert_eq!(policy.backoff_multiplier, decoded.backoff_multiplier);
    assert_eq!(policy.max_backoff_ms, decoded.max_backoff_ms);
}

#[test]
fn test_retry_policy_exponential_backoff() {
    let policy = RetryPolicy {
        max_retries: 5,
        initial_backoff_ms: 100,
        backoff_multiplier: 2.0,
        max_backoff_ms: 10_000,
    };
    assert_eq!(policy.backoff_for_attempt(0), 100);
    assert_eq!(policy.backoff_for_attempt(1), 200);
    assert_eq!(policy.backoff_for_attempt(2), 400);
    assert_eq!(policy.backoff_for_attempt(3), 800);
    assert_eq!(policy.backoff_for_attempt(4), 1600);
}

#[test]
fn test_retry_policy_max_backoff_cap() {
    let policy = RetryPolicy {
        max_retries: 10,
        initial_backoff_ms: 1000,
        backoff_multiplier: 10.0,
        max_backoff_ms: 5000,
    };
    assert_eq!(policy.backoff_for_attempt(0), 1000);
    assert_eq!(policy.backoff_for_attempt(1), 5000); // capped
    assert_eq!(policy.backoff_for_attempt(2), 5000); // capped
}

// ===== PermissionSet (round-trip + allow methods) =====

#[test]
fn test_permission_set_roundtrip() {
    let p = PermissionSet {
        allowed_tools: HashSet::from(["read_file".to_string(), "search_code".to_string()]),
        allowed_paths: HashSet::from([PathBuf::from("/tmp")]),
        max_risk: RiskLevel::L2,
    };
    let json = serde_json::to_string(&p).unwrap();
    let decoded: PermissionSet = serde_json::from_str(&json).unwrap();
    assert_eq!(p.allowed_tools, decoded.allowed_tools);
    assert_eq!(p.allowed_paths, decoded.allowed_paths);
    assert_eq!(p.max_risk, decoded.max_risk);
}

#[test]
fn test_permission_set_default_allows_all() {
    let p = PermissionSet::default();
    // empty allowlist = allow everything
    assert!(p.allows_tool("any_tool"));
    assert!(p.allows_path(&PathBuf::from("/anywhere")));
    assert_eq!(p.max_risk, RiskLevel::L3);
}

#[test]
fn test_permission_set_allows_tool() {
    let p = PermissionSet {
        allowed_tools: HashSet::from(["read_file".to_string()]),
        ..PermissionSet::default()
    };
    assert!(p.allows_tool("read_file"));
    assert!(!p.allows_tool("write_file"));
}

#[test]
fn test_permission_set_allows_path() {
    let p = PermissionSet {
        allowed_paths: HashSet::from([PathBuf::from("/tmp")]),
        ..PermissionSet::default()
    };
    assert!(p.allows_path(&PathBuf::from("/tmp")));
    assert!(!p.allows_path(&PathBuf::from("/etc")));
}

#[test]
fn test_permission_set_allows_risk() {
    let p = PermissionSet {
        max_risk: RiskLevel::L1,
        ..PermissionSet::default()
    };
    assert!(p.allows_risk(RiskLevel::L0));
    assert!(p.allows_risk(RiskLevel::L1));
    assert!(!p.allows_risk(RiskLevel::L2));
    assert!(!p.allows_risk(RiskLevel::L3));
}

// ===== Role (struct + serde) =====

#[test]
fn test_role_roundtrip() {
    let role = Role::new(
        "developer",
        PermissionSet {
            allowed_tools: HashSet::from(["read_file".to_string()]),
            allowed_paths: HashSet::new(),
            max_risk: RiskLevel::L1,
        },
    );
    let json = serde_json::to_string(&role).unwrap();
    let decoded: Role = serde_json::from_str(&json).unwrap();
    assert_eq!(role.name, decoded.name);
    assert_eq!(role.permissions.max_risk, decoded.permissions.max_risk);
}

// ===== Capability (struct + serde) =====

#[test]
fn test_capability_roundtrip() {
    let cap = Capability::new("read_file", "Read a file from disk");
    let json = serde_json::to_string(&cap).unwrap();
    let decoded: Capability = serde_json::from_str(&json).unwrap();
    assert_eq!(cap.name, decoded.name);
    assert_eq!(cap.description, decoded.description);
}

// ===== MemoryCategory (4-variant enum) =====

#[test]
fn test_memory_category_roundtrip() {
    for cat in [
        MemoryCategory::Project,
        MemoryCategory::User,
        MemoryCategory::Snippet,
        MemoryCategory::Tool,
    ] {
        let json = serde_json::to_string(&cat).unwrap();
        let decoded: MemoryCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(cat, decoded);
    }
}

// ===== Importance (Ord + Critical) =====

#[test]
fn test_importance_roundtrip() {
    for imp in [
        Importance::Low,
        Importance::Normal,
        Importance::High,
        Importance::Critical,
    ] {
        let json = serde_json::to_string(&imp).unwrap();
        let decoded: Importance = serde_json::from_str(&json).unwrap();
        assert_eq!(imp, decoded);
    }
}

#[test]
fn test_importance_ordering() {
    assert!(Importance::Low < Importance::Normal);
    assert!(Importance::Normal < Importance::High);
    assert!(Importance::High < Importance::Critical);
    assert_eq!(Importance::default(), Importance::Normal);
}

#[test]
fn test_importance_sort() {
    let mut v = vec![
        Importance::Critical,
        Importance::Low,
        Importance::High,
        Importance::Normal,
    ];
    v.sort();
    assert_eq!(
        v,
        vec![
            Importance::Low,
            Importance::Normal,
            Importance::High,
            Importance::Critical
        ]
    );
}

// ===== UsageStats =====

#[test]
fn test_usage_stats_default() {
    let u = UsageStats::default();
    assert_eq!(u.prompt_tokens, 0);
    assert_eq!(u.completion_tokens, 0);
}
