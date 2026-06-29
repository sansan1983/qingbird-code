use qbird_code_infra::memory::SessionStore;
use qbird_code_models::{EflowError, Message};
use tempfile::TempDir;

fn open_store_in(dir: &TempDir, name: &str) -> SessionStore {
    let path = dir.path().join(name);
    SessionStore::open(&path).expect("open SessionStore")
}

#[test]
fn test_rename_not_found_errors() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "rename_nf.db");
    let err = store.rename("no-such-id", "new").expect_err("should error");
    match err {
        EflowError::SessionNotFound { id } => assert_eq!(id, "no-such-id"),
        other => panic!("expected SessionNotFound, got {:?}", other),
    }
}

#[test]
fn test_rename_clears_name_with_empty_string() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "rename_clear.db");
    store
        .save_messages("s1", &[Message::user("hi")])
        .expect("save");
    store.rename("s1", "initial-name").expect("rename");
    let meta = store.list_with_meta().expect("list");
    assert_eq!(meta[0].name, "initial-name");

    store.rename("s1", "").expect("clear name");
    let meta = store.list_with_meta().expect("list after clear");
    assert_eq!(meta[0].name, "", "empty string should clear the name");
}

#[test]
fn test_delete_exact_id_wins_over_prefix() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "exact_wins.db");
    let archive = dir.path().join("archive");
    store
        .save_messages("abc", &[Message::user("exact")])
        .expect("save exact");
    store
        .save_messages("abc-def", &[Message::user("prefix")])
        .expect("save prefix");

    // "abc" is an exact match for "abc", so it should delete "abc" (not "abc-def")
    store.delete("abc", &archive).expect("delete by exact id");
    let remaining = store.list_with_meta().expect("list");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, "abc-def");
}

#[test]
fn test_save_empty_messages_creates_session() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "empty_msg.db");
    store.save_messages("empty-sess", &[]).expect("save empty");
    let meta = store.list_with_meta().expect("list");
    assert_eq!(meta.len(), 1);
    assert_eq!(meta[0].message_count, 0);
    let loaded = store.load_messages("empty-sess").expect("load");
    assert!(loaded.is_empty());
}

#[test]
fn test_rename_updates_list_with_meta_immediately() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "rename_meta.db");
    store
        .save_messages("s1", &[Message::user("msg")])
        .expect("save");
    store.rename("s1", "display-name").expect("rename");
    let meta = store.list_with_meta().expect("list");
    assert_eq!(meta.len(), 1);
    assert_eq!(meta[0].name, "display-name");
    assert_eq!(meta[0].id, "s1");
}

#[test]
fn test_save_overwrites_existing_messages() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "overwrite.db");
    store
        .save_messages("s1", &[Message::user("v1"), Message::assistant("r1", None)])
        .expect("save v1");
    let loaded = store.load_messages("s1").expect("load v1");
    assert_eq!(loaded.len(), 2);

    store
        .save_messages("s1", &[Message::user("v2")])
        .expect("overwrite");
    let loaded = store.load_messages("s1").expect("load v2");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].content, "v2");
    let meta = store.list_with_meta().expect("list");
    assert_eq!(meta[0].message_count, 1);
}

#[test]
fn test_cleanup_when_fewer_than_keep() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store_in(&dir, "fewer.db");
    for i in 0..5 {
        store
            .save_messages(&format!("s-{}", i), &[Message::user("x")])
            .expect("save");
    }
    let deleted = store.cleanup_old_sessions(50).expect("cleanup");
    assert!(
        deleted.is_empty(),
        "no sessions should be deleted when count < keep"
    );
    assert_eq!(store.list_with_meta().expect("list").len(), 5);
}
