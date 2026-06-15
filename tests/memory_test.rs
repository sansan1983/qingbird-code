rust_i18n::i18n!("locales", fallback = "en-US");

use std::time::{Duration, SystemTime};

use eflow::common::types::{Importance, MemoryCategory};
use eflow::infrastructure::memory::{
    CompositeMemory, MemoryEntry, MemoryManager, ProjectMemory, RecallScope, WorkingMemory,
};

fn entry(content: &str, importance: Importance) -> MemoryEntry {
    MemoryEntry::new(content, MemoryCategory::TaskResult, importance)
}

#[test]
fn test_working_memory_remember_and_recall() {
    let mut wm = WorkingMemory::new(10);
    let id = wm
        .remember(entry("the project uses Rust", Importance::Normal))
        .unwrap();
    assert!(!id.is_nil());

    let results = wm.recall("Rust", RecallScope::Working, 5).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].content.contains("Rust"));
}

#[test]
fn test_working_memory_lru_eviction() {
    let mut wm = WorkingMemory::new(3);
    wm.remember(entry("a", Importance::Normal)).unwrap();
    wm.remember(entry("b", Importance::Normal)).unwrap();
    wm.remember(entry("c", Importance::Normal)).unwrap();
    wm.remember(entry("d", Importance::Normal)).unwrap();

    assert_eq!(wm.len(), 3);
    let all = wm.recall("", RecallScope::Working, 10).unwrap();
    let contents: Vec<&str> = all.iter().map(|e| e.content.as_str()).collect();
    assert!(contents.contains(&"b"));
    assert!(contents.contains(&"c"));
    assert!(contents.contains(&"d"));
    assert!(!contents.contains(&"a"));
}

#[test]
fn test_working_memory_lru_prefers_evicting_low_first() {
    let mut wm = WorkingMemory::new(2);
    wm.remember(entry("important-1", Importance::High)).unwrap();
    wm.remember(entry("trivial-1", Importance::Low)).unwrap();
    wm.remember(entry("new", Importance::Normal)).unwrap();

    let results = wm.recall("", RecallScope::Working, 10).unwrap();
    let contents: Vec<&str> = results.iter().map(|e| e.content.as_str()).collect();
    assert!(contents.contains(&"important-1"), "high importance kept");
    assert!(contents.contains(&"new"));
    assert!(
        !contents.contains(&"trivial-1"),
        "low importance evicted first"
    );
}

#[test]
fn test_working_memory_forget() {
    let mut wm = WorkingMemory::new(10);
    let id = wm.remember(entry("to forget", Importance::Normal)).unwrap();
    assert_eq!(wm.len(), 1);
    wm.forget(id).unwrap();
    assert_eq!(wm.len(), 0);
}

#[test]
fn test_working_memory_cleanup_ttl() {
    let mut wm = WorkingMemory::new(10);
    let mut e = entry("ephemeral", Importance::Low);
    e.ttl = Some(Duration::from_millis(0));
    let id = wm.remember(e).unwrap();
    std::thread::sleep(Duration::from_millis(10));
    let removed = wm.cleanup().unwrap();
    assert_eq!(removed, 1);
    let results = wm.recall("ephemeral", RecallScope::Working, 5).unwrap();
    assert!(results.is_empty());
    let _ = id;
}

#[test]
fn test_working_memory_recall_since() {
    let mut wm = WorkingMemory::new(10);
    let before = SystemTime::now();
    wm.remember(entry("after", Importance::Normal)).unwrap();
    let results = wm.recall_since(before, RecallScope::Working).unwrap();
    assert!(results.iter().any(|e| e.content == "after"));
}

#[test]
fn test_project_memory_in_memory_remember_and_recall_fts() {
    let mut pm = ProjectMemory::in_memory().unwrap();
    pm.remember(entry("the database uses SQLite FTS5", Importance::High))
        .unwrap();
    pm.remember(entry("the API uses REST", Importance::Normal))
        .unwrap();

    let results = pm.recall("SQLite", RecallScope::Project, 5).unwrap();
    assert!(!results.is_empty(), "FTS should find SQLite match");
    assert!(results[0].content.contains("SQLite"));
}

#[test]
fn test_project_memory_in_memory_forget() {
    let mut pm = ProjectMemory::in_memory().unwrap();
    let id = pm.remember(entry("remove me", Importance::Normal)).unwrap();
    pm.forget(id).unwrap();
    let results = pm.recall("remove", RecallScope::Project, 5).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_project_memory_in_memory_cleanup() {
    let mut pm = ProjectMemory::in_memory().unwrap();
    let mut e = entry("ephemeral", Importance::Low);
    e.ttl = Some(Duration::from_secs(0));
    pm.remember(e).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    let removed = pm.cleanup().unwrap();
    assert_eq!(removed, 1);
}

#[test]
fn test_composite_memory_smart_routing() {
    let mut cm = CompositeMemory::in_memory(50).unwrap();

    let id_low = cm
        .remember_smart(entry("low-stakes chatter", Importance::Low))
        .unwrap();
    let id_normal = cm
        .remember_smart(entry("important decision made", Importance::Normal))
        .unwrap();
    assert_ne!(id_low, id_normal);

    let hits = cm.recall_smart("decision", 10).unwrap();
    assert!(hits.iter().any(|e| e.content.contains("decision")));
}

#[test]
fn test_composite_memory_recall_limit() {
    let mut cm = CompositeMemory::in_memory(50).unwrap();
    for i in 0..5 {
        cm.remember_smart(entry(&format!("note {}", i), Importance::Normal))
            .unwrap();
    }
    let hits = cm.recall_smart("note", 3).unwrap();
    assert!(hits.len() <= 3);
}
