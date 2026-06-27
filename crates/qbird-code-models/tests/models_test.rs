use qbird_code_models::{
    ActionResult, FeedbackRecord, Importance, IntentType, MemoryCategory, Message, MessageRole,
    PermissionSet, PlannedStep, QualityVerdict, RetryPolicy, RiskLevel, Role, TaskPriority,
    TaskSpec, ToolCall, ToolCallFunction, ToolCallSummary, UsageStats,
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

// ===== TaskSpec =====

#[test]
fn test_task_spec_new() {
    let spec = TaskSpec::new("Analyze code".into(), RiskLevel::L1);
    assert_eq!(spec.description, "Analyze code");
    assert_eq!(spec.risk_level, RiskLevel::L1);
    assert_eq!(spec.priority, TaskPriority::Normal);
    assert_eq!(spec.timeout_secs, 300);
}

// ===== RetryPolicy =====

#[test]
fn test_retry_policy_default() {
    let policy = RetryPolicy::default();
    assert_eq!(policy.max_retries, 3);
    assert_eq!(policy.backoff_ms, 1000);
}

// ===== PermissionSet =====

#[test]
fn test_permission_set_default() {
    let p = PermissionSet::default();
    assert!(!p.network_enabled);
    assert_eq!(p.max_file_size_bytes, 10 * 1024 * 1024);
}

// ===== FeedbackRecord =====

#[test]
fn test_feedback_record_now() {
    let verdict = QualityVerdict::Pass {
        summary: "OK".into(),
    };
    let record = FeedbackRecord::now(2, verdict);
    assert_eq!(record.retry_count, 2);
}

// ===== QualityVerdict serialization =====

#[test]
fn test_quality_verdict_serde() {
    let v = QualityVerdict::Rework {
        reason: "bad".into(),
        suggestion: "fix it".into(),
    };
    let json = serde_json::to_string(&v).unwrap();
    let decoded: QualityVerdict = serde_json::from_str(&json).unwrap();
    match decoded {
        QualityVerdict::Rework { reason, .. } => assert_eq!(reason, "bad"),
        _ => panic!("expected Rework"),
    }
}

// ===== UsageStats =====

#[test]
fn test_usage_stats_default() {
    let u = UsageStats::default();
    assert_eq!(u.prompt_tokens, 0);
    assert_eq!(u.completion_tokens, 0);
}

// ===== IntentType =====

#[test]
fn test_intent_type_codes() {
    assert_ne!(IntentType::CodeReview as u8, IntentType::Chat as u8);
}

// ===== Struct round-trips =====

#[test]
fn test_action_result_serde() {
    let r = ActionResult {
        success: true,
        output: "done".into(),
        tool_calls: vec![ToolCallSummary {
            tool_name: "read_file".into(),
            success: true,
            duration_ms: 10,
            summary: "OK".into(),
        }],
        duration_ms: 100,
    };
    let json = serde_json::to_string(&r).unwrap();
    let decoded: ActionResult = serde_json::from_str(&json).unwrap();
    assert!(decoded.success);
    assert_eq!(decoded.tool_calls.len(), 1);
}

#[test]
fn test_planned_step_depends_on() {
    let step = PlannedStep {
        order: 2,
        action: "analyze".into(),
        tool: "search_code".into(),
        params: serde_json::json!({"pattern": "fn main"}),
        depends_on: Some(1),
    };
    assert_eq!(step.depends_on, Some(1));
}

// ===== Enum completeness =====

#[test]
fn test_role_variants() {
    let roles = [
        (Role::FileAssistant, "FileAssistant"),
        (Role::CodeAssistant, "CodeAssistant"),
        (Role::DataAnalyst, "DataAnalyst"),
        (Role::Generalist, "Generalist"),
    ];
    for (role, name) in roles {
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, format!("\"{name}\""));
    }
}

#[test]
fn test_memory_category_variants() {
    let categories = [
        MemoryCategory::TaskResult,
        MemoryCategory::Decision,
        MemoryCategory::UserPreference,
    ];
    for cat in &categories {
        let json = serde_json::to_string(cat).unwrap();
        let decoded: MemoryCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(cat, &decoded);
    }
}

#[test]
fn test_importance_serde() {
    for imp in [
        Importance::Low,
        Importance::Normal,
        Importance::High,
        Importance::Pinned,
    ] {
        let json = serde_json::to_string(&imp).unwrap();
        let decoded: Importance = serde_json::from_str(&json).unwrap();
        assert_eq!(imp, decoded);
    }
}
