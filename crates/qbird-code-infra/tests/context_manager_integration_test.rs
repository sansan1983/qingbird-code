use qbird_code_infra::memory::ContextManager;
use qbird_code_models::Message;

// ===== add_chat_message: convert Message → (role, content) =====

#[test]
fn test_add_chat_message_from_user_message() {
    let mut cm = ContextManager::new("sess".into(), 32000);
    let msg = Message::user("hello there");
    cm.add_chat_message(&msg);
    assert_eq!(cm.get_message_count(), 1);
    assert_eq!(cm.get_messages()[0].role, "user");
    assert_eq!(cm.get_messages()[0].content, "hello there");
}

#[test]
fn test_add_chat_message_from_assistant() {
    let mut cm = ContextManager::new("sess".into(), 32000);
    cm.add_chat_message(&Message::assistant("reply", None));
    cm.add_chat_message(&Message::user("another question"));
    assert_eq!(cm.get_message_count(), 2);
    assert_eq!(cm.get_messages()[0].role, "assistant");
    assert_eq!(cm.get_messages()[1].role, "user");
}

#[test]
fn test_add_chat_message_from_tool_result() {
    let mut cm = ContextManager::new("sess".into(), 32000);
    let tool_msg = Message::tool_result("call_1".into(), "read_file".into(), "file content");
    cm.add_chat_message(&tool_msg);
    assert_eq!(cm.get_message_count(), 1);
    assert_eq!(cm.get_messages()[0].role, "tool");
}

#[test]
fn test_add_chat_message_from_system() {
    let mut cm = ContextManager::new("sess".into(), 32000);
    cm.add_chat_message(&Message::system("You are helpful"));
    assert_eq!(cm.get_message_count(), 1);
    assert_eq!(cm.get_messages()[0].role, "system");
}

// ===== get_messages: simple Vec<ContextMessage> accessor =====

#[test]
fn test_get_messages_returns_cloneable_vec() {
    let mut cm = ContextManager::new("sess".into(), 32000);
    cm.add_chat_message(&Message::user("m1"));
    let m = cm.get_messages();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].content, "m1");
}

// ===== get_messages_within_budget truncation =====

#[test]
fn test_get_within_budget_truncates_old_messages() {
    // Add 10 messages of 100 chars each (~ 33 tokens each = ~330 tokens total).
    // Budget of 100 tokens can only fit ~3 messages from the end.
    let mut cm = ContextManager::new("sess".into(), 32000);
    for i in 0..10 {
        let content = "x".repeat(100);
        cm.add_chat_message(&Message::user(format!("msg{i} {content}")));
    }
    let within = cm.get_messages_within_budget(100);
    assert!(
        within.len() < 10,
        "budget=100 should truncate; got {} messages",
        within.len()
    );
    assert!(!within.is_empty(), "should keep at least one message");
}

#[test]
fn test_get_within_budget_keeps_recent_messages() {
    // 5 small messages fit easily in 1000 tokens; none should be dropped.
    let mut cm = ContextManager::new("sess".into(), 32000);
    for i in 0..5 {
        cm.add_chat_message(&Message::user(format!("msg{i}")));
    }
    let within = cm.get_messages_within_budget(1000);
    assert_eq!(within.len(), 5, "all 5 small messages should fit");
}

// ===== checkpoint threshold =====

#[test]
fn test_checkpoint_fires_at_threshold() {
    let mut cm = ContextManager::new("sess".into(), 1000);
    cm.set_threshold(0.8);
    // ~5000 chars at 4 chars/token ≈ 1250 tokens; with limit 1000 → 1.25 > 0.8
    cm.add_chat_message(&Message::user("A".repeat(5000)));
    let event = cm.checkpoint_if_needed();
    assert!(
        event.is_some(),
        "checkpoint should fire at >= 80% threshold"
    );
    let event = event.unwrap();
    assert!(event.token_count > 800);
}

#[test]
fn test_checkpoint_does_not_fire_below_threshold() {
    let mut cm = ContextManager::new("sess".into(), 1000);
    cm.set_threshold(0.8);
    cm.add_chat_message(&Message::user("small"));
    assert!(cm.checkpoint_if_needed().is_none());
}

// ===== disabled (cm = None) safety =====
// Note: this is mainly an integration concern (ReactLoop.run should
// handle Option<None> gracefully). Unit test: ensure the helper
// functions are no-ops when given an empty manager, which is the
// closest we can get to a "None" case here.

#[test]
fn test_disabled_empty_manager_no_checkpoint() {
    // A freshly-constructed manager with no messages is the "disabled"
    // semantic equivalent: it never fires, never errors.
    let mut cm = ContextManager::new("sess".into(), 32000);
    assert_eq!(cm.get_message_count(), 0);
    assert!(cm.checkpoint_if_needed().is_none());
    assert!(cm.get_messages_within_budget(1000).is_empty());
}
