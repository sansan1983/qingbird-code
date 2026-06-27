use qbird_code_models::EflowError;

#[test]
fn test_user_message_config() {
    let err = EflowError::Config("bad yaml".into());
    let msg = err.user_message();
    assert!(!msg.is_empty());
    assert!(msg.contains("bad yaml"));
}

#[test]
fn test_user_message_llm_provider() {
    let err = EflowError::LlmProvider("timeout".into());
    let msg = err.user_message();
    assert!(!msg.is_empty());
    assert!(msg.contains("timeout"));
}

#[test]
fn test_user_message_memory() {
    let err = EflowError::Memory("db locked".into());
    let msg = err.user_message();
    assert!(!msg.is_empty());
    assert!(msg.contains("db locked"));
}

#[test]
fn test_user_message_profile_not_found() {
    let err = EflowError::ProfileNotFound("developer".into());
    let msg = err.user_message();
    assert!(!msg.is_empty());
    assert!(msg.contains("developer"));
}

#[test]
fn test_user_message_llm_auth_failed() {
    let err = EflowError::LlmAuthFailed("401".into());
    let msg = err.user_message();
    assert!(!msg.is_empty());
    assert!(msg.contains("401"));
}

#[test]
fn test_user_message_skill_not_found() {
    let err = EflowError::SkillNotFound("sdd".into());
    let msg = err.user_message();
    assert!(!msg.is_empty());
    assert!(msg.contains("sdd"));
}

#[test]
fn test_user_message_permission_denied() {
    let err = EflowError::PermissionDenied("read /etc".into());
    let msg = err.user_message();
    assert!(!msg.is_empty());
    assert!(msg.contains("read /etc"));
}

#[test]
fn test_user_message_internal_localized() {
    let err = EflowError::Internal("oops".into());
    let msg = err.user_message();
    // The inserted string is locale-agnostic; verify it survives i18n
    // regardless of which prefix the active locale resolves to.
    assert!(msg.contains("oops"));
    assert!(!msg.is_empty());
}

#[test]
fn test_user_message_rate_limited_localized() {
    let err = EflowError::RateLimited("openai".into());
    let msg = err.user_message();
    assert!(msg.contains("openai"));
    assert!(!msg.is_empty());
}

#[test]
fn test_user_message_task_cancelled_localized() {
    let err = EflowError::TaskCancelled("task-1".into());
    let msg = err.user_message();
    assert!(msg.contains("task-1"));
    assert!(!msg.is_empty());
}

#[test]
fn test_user_message_io() {
    let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing-file");
    let err = EflowError::Io(io);
    let msg = err.user_message();
    assert!(msg.contains("missing-file"));
    assert!(!msg.is_empty());
}

#[test]
fn test_user_message_serialization() {
    let err = EflowError::Serialization("bad json".into());
    let msg = err.user_message();
    assert!(msg.contains("bad json"));
    assert!(!msg.is_empty());
}

#[test]
fn test_user_message_tool() {
    let err = EflowError::Tool("read_file: path empty".into());
    let msg = err.user_message();
    assert!(msg.contains("read_file: path empty"));
    assert!(!msg.is_empty());
}

#[test]
fn test_user_message_risk_escalated() {
    let err = EflowError::RiskEscalated {
        task_id: "task-42".into(),
        reason: "dangerous write".into(),
    };
    let msg = err.user_message();
    assert!(msg.contains("task-42"));
    assert!(msg.contains("dangerous write"));
    assert!(!msg.is_empty());
}

#[test]
fn test_user_message_all_variants_non_empty() {
    // Smoke test: every variant produces a non-empty message (i.e., every
    // arm of the match is wired and the i18n key resolves). Catches the
    // "added a new variant but forgot to wire user_message" failure mode.
    let cases: Vec<EflowError> = vec![
        EflowError::Config("x".into()),
        EflowError::LlmProvider("x".into()),
        EflowError::RateLimited("x".into()),
        EflowError::LlmAuthFailed("x".into()),
        EflowError::Memory("x".into()),
        EflowError::Tool("x".into()),
        EflowError::RiskEscalated {
            task_id: "x".into(),
            reason: "y".into(),
        },
        EflowError::TaskCancelled("x".into()),
        EflowError::ProfileNotFound("x".into()),
        EflowError::SkillNotFound("x".into()),
        EflowError::PermissionDenied("x".into()),
        EflowError::Io(std::io::Error::other("x")),
        EflowError::Serialization("x".into()),
        EflowError::Internal("x".into()),
    ];
    assert_eq!(cases.len(), 14);
    for e in cases {
        assert!(!e.user_message().is_empty(), "empty user_message for {e:?}");
    }
}
