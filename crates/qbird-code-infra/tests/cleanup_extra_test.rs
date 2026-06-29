use qbird_code_infra::memory::SessionStore;
use tempfile::TempDir;

fn open_store(dir: &TempDir) -> SessionStore {
    let path = dir.path().join("throttle_extra.db");
    SessionStore::open(&path).expect("open SessionStore")
}

#[test]
fn test_mark_cleanup_twice_still_throttled() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store(&dir);
    store.mark_cleanup().expect("first mark");
    store.mark_cleanup().expect("second mark should not error");
    assert!(
        !store.should_cleanup(24).expect("should_cleanup"),
        "should still be throttled after double mark"
    );
}

#[test]
fn test_should_cleanup_with_1h_interval() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store(&dir);
    store.mark_cleanup().expect("mark");
    // Right after marking, 1h interval should also be throttled
    assert!(
        !store.should_cleanup(1).expect("1h"),
        "should be throttled within 1h"
    );
}

#[test]
fn test_should_cleanup_0h_interval_always_runs() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store(&dir);
    store.mark_cleanup().expect("mark");
    // 0h interval means "always run"
    assert!(
        store.should_cleanup(0).expect("0h"),
        "0h interval should always return true"
    );
}
