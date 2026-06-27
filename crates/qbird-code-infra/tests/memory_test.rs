use qbird_code_infra::memory::{MemoryEntry, MemoryManager};

#[test]
fn test_memory_save_and_search() {
    let tmp = std::env::temp_dir().join("qbird_memory_test.db");
    let _ = std::fs::remove_file(&tmp);

    let mm = MemoryManager::open(&tmp).expect("open memory DB");

    let entry = MemoryEntry {
        path: "/test/file.rs".into(),
        scope: "project".into(),
        scope_id: Some("test-proj".into()),
        r#type: "code".into(),
        body: "fn hello() { println!(\"Hello\"); }".into(),
        fingerprint: "abc123".into(),
        last_indexed_at: 1234567890,
    };

    let status = mm.save(&entry).expect("save entry");
    assert_eq!(status, "created");

    let results = mm.search("hello", None).expect("search");
    assert!(!results.is_empty(), "should find results");

    let status2 = mm.save(&entry).expect("save again");
    assert_eq!(status2, "unchanged");

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_memory_search_empty_query() {
    let tmp = std::env::temp_dir().join("qbird_memory_test_empty.db");
    let _ = std::fs::remove_file(&tmp);

    let mm = MemoryManager::open(&tmp).expect("open memory DB");
    let results = mm.search("", None).expect("search empty");
    assert!(results.is_empty(), "empty query returns no results");

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_session_store_save_and_load() {
    let tmp = std::env::temp_dir().join("qbird_session_test.db");
    let _ = std::fs::remove_file(&tmp);

    let store = qbird_code_infra::memory::SessionStore::open(&tmp).expect("open store");
    let messages = vec![
        qbird_code_models::Message::user("Hello"),
        qbird_code_models::Message::assistant("Hi there", None),
    ];

    store.save_messages("sess_1", &messages).expect("save");
    let loaded = store.load_messages("sess_1").expect("load");
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].content, "Hello");

    let sessions = store.list_sessions().expect("list");
    assert_eq!(sessions.len(), 1);

    let _ = std::fs::remove_file(&tmp);
}
