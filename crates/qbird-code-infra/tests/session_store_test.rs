use qbird_code_infra::memory::{SessionMeta, SessionStore};
use qbird_code_models::{EflowError, Message};
use tempfile::TempDir;

fn open_store_in(dir: &TempDir, name: &str) -> SessionStore {
    let path = dir.path().join(name);
    SessionStore::open(&path).expect("open SessionStore")
}

#[test]
fn test_delete_writes_archive() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "del_archive.db");
    let archive = dir.path().join("archive");
    let session_id = "sess-archive-1";
    let messages = vec![
        Message::user("first"),
        Message::assistant("second", None),
        Message::user("third"),
    ];
    store.save_messages(session_id, &messages).expect("save");

    store.delete(session_id, &archive).expect("delete");

    let archive_file = archive.join(format!("{}.jsonl", session_id));
    assert!(archive_file.exists(), "archive file should exist");

    let content = std::fs::read_to_string(&archive_file).expect("read archive");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 3, "archive should have 3 lines");
    assert!(lines[0].contains("\"role\":\"user\""));
    assert!(lines[0].contains("\"content\":\"first\""));
    assert!(lines[1].contains("\"role\":\"assistant\""));
    assert!(lines[2].contains("\"content\":\"third\""));

    let after = store.load_messages(session_id).expect("load after delete");
    assert!(after.is_empty(), "messages should be gone after delete");
}

#[test]
fn test_delete_prefix_match() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "del_prefix.db");
    let archive = dir.path().join("archive");
    store
        .save_messages("aaa-111", &[Message::user("A")])
        .expect("save aaa-111");
    store
        .save_messages("aaa-222", &[Message::user("B")])
        .expect("save aaa-222");

    store.delete("aaa-1", &archive).expect("delete by prefix");

    let remaining = store.list_with_meta().expect("list");
    let ids: Vec<&str> = remaining.iter().map(|m| m.id.as_str()).collect();
    assert!(!ids.contains(&"aaa-111"), "aaa-111 should be deleted");
    assert!(ids.contains(&"aaa-222"), "aaa-222 should remain");
}

#[test]
fn test_rename_persists() {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("rename.db");
    let session_id = "sess-rename";

    {
        let store = SessionStore::open(&db_path).expect("open first");
        store
            .save_messages(session_id, &[Message::user("hi")])
            .expect("save");
        store.rename(session_id, "new label").expect("rename");
    }

    let store2 = SessionStore::open(&db_path).expect("open second");
    let list = store2.list_with_meta().expect("list");
    let meta = list
        .iter()
        .find(|m| m.id == session_id)
        .expect("session should be present after reopen");
    assert_eq!(meta.name, "new label");
}

#[test]
fn test_list_with_meta_sorted() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "list_meta.db");

    for i in 0..3 {
        let id = format!("sess-{}", i);
        store
            .save_messages(&id, &[Message::user(format!("msg-{}", i))])
            .expect("save");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let list: Vec<SessionMeta> = store.list_with_meta().expect("list_with_meta");
    assert_eq!(list.len(), 3, "should return all 3 with no LIMIT 20");
    for (i, m) in list.iter().enumerate().take(3) {
        assert!(!m.id.is_empty());
        assert_eq!(m.message_count, 1, "message_count for {} should be 1", i);
        assert!(m.created_at > 0 && m.updated_at > 0);
    }
    for window in list.windows(2) {
        assert!(
            window[0].updated_at >= window[1].updated_at,
            "list should be DESC by updated_at"
        );
    }
}

#[test]
fn test_cleanup_keeps_n() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "cleanup.db");

    for i in 0..60 {
        let id = format!("sess-{:03}", i);
        store
            .save_messages(&id, &[Message::user(format!("msg-{}", i))])
            .expect("save");
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    let deleted = store.cleanup_old_sessions(50).expect("cleanup");
    assert_eq!(deleted.len(), 10, "should delete 10 oldest");
    assert_eq!(store.list_with_meta().expect("list").len(), 50);
}

#[test]
fn test_archive_jsonl_format() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "archive_fmt.db");
    let archive = dir.path().join("archive");
    let session_id = "sess-fmt";
    let messages = vec![
        Message::system("sys"),
        Message::user("u1"),
        Message::assistant("a1", None),
    ];
    store.save_messages(session_id, &messages).expect("save");
    store.delete(session_id, &archive).expect("delete");

    let archive_file = archive.join(format!("{}.jsonl", session_id));
    let content = std::fs::read_to_string(&archive_file).expect("read");
    for line in content.lines() {
        let parsed: serde_json::Value = serde_json::from_str(line).expect("valid JSON line");
        assert!(parsed.get("role").is_some(), "line missing role: {}", line);
        assert!(
            parsed.get("content").is_some(),
            "line missing content: {}",
            line
        );
        assert!(
            parsed.get("timestamp").is_some(),
            "line missing timestamp: {}",
            line
        );
        assert!(
            parsed["timestamp"].is_number(),
            "timestamp must be numeric: {}",
            line
        );
    }
}

#[test]
fn test_delete_not_found_errors() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "del_nf.db");
    let archive = dir.path().join("archive");

    let err = store
        .delete("does-not-exist", &archive)
        .expect_err("should error");
    match err {
        EflowError::SessionNotFound { id } => assert_eq!(id, "does-not-exist"),
        other => panic!("expected SessionNotFound, got {:?}", other),
    }
}

#[test]
fn test_delete_ambiguous_prefix_errors() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "del_ambig.db");
    let archive = dir.path().join("archive");

    store
        .save_messages("abc-111", &[Message::user("1")])
        .expect("save abc-111");
    store
        .save_messages("abc-222", &[Message::user("2")])
        .expect("save abc-222");

    let err = store.delete("abc", &archive).expect_err("should error");
    match err {
        EflowError::SessionAmbiguous { prefix, count } => {
            assert_eq!(prefix, "abc");
            assert_eq!(count, 2);
        }
        other => panic!("expected SessionAmbiguous, got {:?}", other),
    }
}
