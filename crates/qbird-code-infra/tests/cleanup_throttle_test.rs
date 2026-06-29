use qbird_code_infra::memory::SessionStore;
use tempfile::TempDir;

fn open_store(dir: &TempDir) -> SessionStore {
    let path = dir.path().join("throttle.db");
    SessionStore::open(&path).expect("open SessionStore")
}

#[test]
fn test_cleanup_first_run_always_runs() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store(&dir);
    assert!(
        store.should_cleanup(24).expect("should_cleanup"),
        "first run with no last_cleanup_at should return true"
    );
}

#[test]
fn test_cleanup_throttled_24h() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store(&dir);
    store.mark_cleanup().expect("mark_cleanup");
    assert!(
        !store.should_cleanup(24).expect("should_cleanup"),
        "should be throttled right after mark_cleanup with 24h interval"
    );
}

#[test]
fn test_cleanup_runs_after_24h() {
    let dir = TempDir::new().expect("tempdir");
    let store = open_store(&dir);
    store.mark_cleanup().expect("mark_cleanup");

    // Overwrite last_cleanup_at to 25 hours ago
    let past = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - 25 * 3600;
    let db_path = dir.path().join("throttle.db");
    let conn = rusqlite::Connection::open(&db_path).expect("open raw conn");
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('last_cleanup_at', ?1)
         ON CONFLICT(key) DO UPDATE SET value = ?1",
        rusqlite::params![past.to_string()],
    )
    .expect("overwrite timestamp");

    assert!(
        store.should_cleanup(24).expect("should_cleanup"),
        "should return true after 25 hours have passed"
    );
}
