//! Comprehensive end-to-end test for Phase 3 (Task 30-10).
//!
//! Covers the full chain: session lifecycle → profile → edit → undo → delete → usage format.

use std::sync::{Arc, Mutex};

use qbird_code_infra::config::{estimate_cost, format_cost};
use qbird_code_infra::memory::SessionStore;
use qbird_code_infra::profile::Profile;
use qbird_code_models::Message;
use qbird_code_tools::{EditTool, ToolRegistry, UndoStack};

#[tokio::test]
async fn test_v03_e2e_full_chain() {
    // === 1. Fresh SessionStore ===
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("e2e_sessions.db");
    let archive_dir = dir.path().join("archive");
    let profile_dir = dir.path().join("profiles");

    let store = SessionStore::open(&db_path).expect("open store");
    assert!(
        store.list_with_meta().expect("list").is_empty(),
        "fresh store should be empty"
    );

    // === 2. Create and load a developer profile ===
    std::fs::create_dir_all(&profile_dir).unwrap();
    let dev_yaml = r#"name: developer
description: "E2E test developer profile"
system_prompt: "You are a Rust developer."
tools_allow: []
risk_threshold: L3
"#;
    std::fs::write(profile_dir.join("developer.yaml"), dev_yaml).unwrap();

    let profile = Profile::load(&profile_dir, "developer").expect("load profile");
    assert_eq!(profile.name, "developer");
    assert_eq!(
        profile.description.as_deref(),
        Some("E2E test developer profile")
    );

    // Apply profile merge
    let mut system_prompt = String::new();
    let mut allowed_tools: Option<Vec<String>> = None;
    let mut risk: Option<String> = None;
    let mut provider = String::from("deepseek");
    let mut model = String::from("deepseek-v4-pro");
    let mut warnings: Vec<String> = Vec::new();
    profile.merge_into(
        &mut system_prompt,
        &mut allowed_tools,
        &mut risk,
        &mut provider,
        &mut model,
        &mut warnings,
    );
    assert_eq!(system_prompt, "You are a Rust developer.");
    assert!(
        warnings.is_empty(),
        "no provider/model change → no warnings"
    );

    // === 3. Save a session with messages ===
    let session_id = "e2e-session-001";
    let messages = vec![
        Message::system("You are a Rust developer."),
        Message::user("Fix the bug in main.rs"),
        Message::assistant("I'll fix it now.", None),
    ];
    store.save_messages(session_id, &messages).expect("save");
    let meta = store.list_with_meta().expect("list");
    assert_eq!(meta.len(), 1);
    assert_eq!(meta[0].id, session_id);
    assert_eq!(meta[0].message_count, 3);

    // === 4. Execute an edit on a temp file ===
    let file_path = dir.path().join("target.txt");
    let original_content = "fn main() {\n    println!(\"bug\");\n}\n";
    std::fs::write(&file_path, original_content).expect("write file");

    let undo_stack: Arc<Mutex<UndoStack>> = Arc::new(Mutex::new(UndoStack::new()));
    let edit_tool = EditTool::new().with_undo_stack(Arc::clone(&undo_stack));
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(edit_tool));

    // Edit: replace "bug" with "hello"
    let edit_out = registry
        .execute(
            "edit",
            serde_json::json!({
                "path": file_path.to_string_lossy(),
                "old_string": "bug",
                "new_string": "hello",
            }),
            uuid::Uuid::new_v4(),
        )
        .await
        .expect("edit succeeds");
    assert!(edit_out.success);
    assert!(edit_out.content.contains("old:"));
    assert!(edit_out.content.contains("new:"));

    let edited = std::fs::read_to_string(&file_path).expect("read after edit");
    assert_eq!(edited, "fn main() {\n    println!(\"hello\");\n}\n");

    // Undo stack has the original content
    assert_eq!(undo_stack.lock().expect("lock").len(), 1);

    // === 5. Undo the edit ===
    let entry = undo_stack.lock().expect("lock").pop().expect("entry");
    assert_eq!(entry.path, file_path);
    assert_eq!(entry.previous_content, original_content);
    std::fs::write(&entry.path, &entry.previous_content).expect("restore");

    let restored = std::fs::read_to_string(&file_path).expect("read after undo");
    assert_eq!(
        restored, original_content,
        "file should be restored to original"
    );
    assert!(undo_stack.lock().expect("lock").is_empty());

    // === 6. Rename session ===
    store.rename(session_id, "bug-fix-session").expect("rename");
    let meta = store.list_with_meta().expect("list after rename");
    assert_eq!(meta[0].name, "bug-fix-session");

    // === 7. Delete session → archived ===
    store.delete(session_id, &archive_dir).expect("delete");
    assert!(
        store
            .list_with_meta()
            .expect("list after delete")
            .is_empty(),
        "store should be empty after delete"
    );

    let archive_file = archive_dir.join(format!("{}.jsonl", session_id));
    assert!(archive_file.exists(), "archive should be created");
    let archive_content = std::fs::read_to_string(&archive_file).expect("read archive");
    let lines: Vec<&str> = archive_content.lines().collect();
    assert_eq!(lines.len(), 3, "archive should have 3 lines (3 messages)");
    for line in &lines {
        let parsed: serde_json::Value = serde_json::from_str(line).expect("valid JSON");
        assert!(parsed.get("role").is_some());
        assert!(parsed.get("content").is_some());
        assert!(parsed.get("timestamp").is_some());
    }

    // === 8. Check cost/usage format ===
    // estimate_cost with known rates
    let cost = estimate_cost(1000, 500, 100, 2.0, 4.0).unwrap();
    // effective_input = 1000 - 100 = 900; cost = 900/1M * 2.0 + 500/1M * 4.0
    let expected = (900.0 / 1_000_000.0) * 2.0 + (500.0 / 1_000_000.0) * 4.0;
    assert!((cost - expected).abs() < 1e-10);

    // format_cost USD
    let usd_str = format_cost(cost, false);
    assert!(usd_str.contains("USD"), "USD format: {usd_str}");
    assert!(usd_str.starts_with("≈ $"));

    // format_cost RMB
    let rmb_str = format_cost(cost, true);
    assert!(rmb_str.starts_with("≈ ¥"), "RMB format: {rmb_str}");

    // estimate_cost returns None for unknown (zero rates)
    assert!(estimate_cost(1000, 500, 0, 0.0, 0.0).is_none());

    // === 9. Profile list ===
    let profiles = Profile::list(&profile_dir).expect("list profiles");
    assert_eq!(profiles, vec!["developer"]);

    // === 10. Cleanup ===
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&archive_dir);
}
