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
fn test_user_message_internal_falls_back_to_display() {
    let err = EflowError::Internal("oops".into());
    let msg = err.user_message();
    // Internal variant has no user-facing key, falls back to Display (English).
    assert!(msg.contains("internal error"));
    assert!(msg.contains("oops"));
}

#[test]
fn test_user_message_rate_limited_falls_back_to_display() {
    let err = EflowError::RateLimited("openai".into());
    let msg = err.user_message();
    assert!(msg.contains("rate limited"));
    assert!(msg.contains("openai"));
}

#[test]
fn test_user_message_task_cancelled_falls_back_to_display() {
    let err = EflowError::TaskCancelled("task-1".into());
    let msg = err.user_message();
    assert!(msg.contains("task cancelled"));
    assert!(msg.contains("task-1"));
}
