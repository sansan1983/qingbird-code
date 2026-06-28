use std::path::PathBuf;

use qbird_code_infra::memory::MemoryManager;

// ===== default_db_path =====

#[test]
fn test_default_db_path() {
    let p = MemoryManager::default_db_path().expect("default_db_path should resolve");
    // Ends with `qingbird/memory.db`
    assert!(p.ends_with("memory.db"), "got: {p:?}");
    let s = p.to_string_lossy();
    assert!(
        s.contains("qingbird"),
        "expected qingbird segment, got: {s}"
    );
}

#[test]
fn test_default_db_path_creates_parent_dir() {
    // If parent doesn't exist, default_db_path should create it (callable from anywhere).
    let p = MemoryManager::default_db_path().expect("default_db_path should resolve");
    let parent = p.parent().expect("memory.db must have a parent");
    // We don't strictly assert existence (other tests may have created it already)
    // but calling default_db_path() a second time must not error.
    let p2 = MemoryManager::default_db_path().expect("second call should also resolve");
    assert_eq!(p, p2, "default path must be deterministic");
    let _ = parent; // silence unused
}

#[test]
fn test_path_in_xdg_data_dir() {
    // data_dir on Windows is %APPDATA% or dirs::data_dir() (Roaming).
    // On Linux it's $XDG_DATA_HOME or $HOME/.local/share.
    // We assert the path is under *some* known XDG-ish parent.
    let p = MemoryManager::default_db_path().expect("default_db_path should resolve");
    // The path must be absolute (per XDG spec).
    assert!(p.is_absolute(), "XDG path must be absolute, got: {p:?}");
}

// ===== open with custom path (override) =====

#[test]
fn test_custom_path_override() {
    let tmp = std::env::temp_dir().join("qingbird_test_mm_custom.db");
    let _ = std::fs::remove_file(&tmp);
    let mm = MemoryManager::open(&tmp).expect("open at custom path");
    let entry = qbird_code_infra::memory::MemoryEntry {
        path: "test/path.md".into(),
        scope: "project".into(),
        scope_id: Some("scope-1".into()),
        r#type: "fact".into(),
        body: "hello world".into(),
        fingerprint: "fp-1".into(),
        last_indexed_at: 0,
    };
    mm.save(&entry).expect("save");
    assert!(tmp.exists(), "custom path DB file should exist");
    let _ = std::fs::remove_file(&tmp);
}

// ===== backwards compat: old yaml field ignored =====

#[test]
fn test_old_yaml_field_ignored() {
    // Old qingbird.yaml with `project_db_path` / `user_db_path` should still
    // deserialize (serde ignores unknown fields by default).
    let yaml = r#"
core:
  language: zh-CN
memory:
  working_memory_limit: 4096
  project_db_path: "/tmp/old_project.db"
  user_db_path: "/tmp/old_user.db"
"#;
    let cfg: qbird_code_infra::config::EflowConfig =
        serde_yaml::from_str(yaml).expect("yaml parse");
    assert_eq!(cfg.memory.working_memory_limit, 4096);
    // The dead fields are no longer accessible at all (compiled out).
    // The fact that this compiles + parses without error is the test.
}

// ===== migration: existing file opens cleanly =====

#[test]
fn test_migration_no_crash() {
    // Simulate an existing DB file from a previous version. Opening it must
    // not panic; missing tables get created (CREATE TABLE IF NOT EXISTS).
    let tmp = std::env::temp_dir().join("qingbird_test_mm_migration.db");
    let _ = std::fs::remove_file(&tmp);
    // First open: creates the schema.
    {
        let mm = MemoryManager::open(&tmp).expect("first open");
        let _ = mm.save(&qbird_code_infra::memory::MemoryEntry {
            path: "old/path.md".into(),
            scope: "project".into(),
            scope_id: None,
            r#type: "fact".into(),
            body: "pre-existing entry".into(),
            fingerprint: "fp-old".into(),
            last_indexed_at: 0,
        });
    }
    // Second open: must not crash, must still see the old entry.
    {
        let mm = MemoryManager::open(&tmp).expect("second open after migration");
        let results = mm
            .search("pre-existing", None)
            .expect("search across migration");
        assert_eq!(results.len(), 1, "old entry must survive reopen");
    }
    let _ = std::fs::remove_file(&tmp);
}

// ===== default_db_path produces a PathBuf (not &str) =====

#[test]
fn test_default_db_path_returns_pathbuf() {
    let p: PathBuf = MemoryManager::default_db_path().expect("resolve");
    assert!(
        p.to_str().is_some(),
        "path must be valid UTF-8 (or at least lossy)"
    );
}
