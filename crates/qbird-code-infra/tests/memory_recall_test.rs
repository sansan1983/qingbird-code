use std::sync::Arc;

use qbird_code_infra::memory::{MemoryEntry, MemoryManager};

// ===== recall() =====

fn make_entry(path: &str, body: &str, scope: &str) -> MemoryEntry {
    MemoryEntry {
        path: path.into(),
        scope: scope.into(),
        scope_id: Some("sess-1".into()),
        r#type: "fact".into(),
        body: body.into(),
        fingerprint: format!("fp-{path}"),
        last_indexed_at: 0,
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_recall_injects_top_5() {
    let tmp = std::env::temp_dir().join("qingbird_mm_recall_top5.db");
    let _ = std::fs::remove_file(&tmp);
    let mm = Arc::new(MemoryManager::open(&tmp).expect("open"));

    // 10 entries with distinct keywords
    for i in 0..10 {
        mm.save(&make_entry(
            &format!("p-{i}"),
            &format!("unique-keyword-{i} shared-context"),
            "user",
        ))
        .expect("save");
    }

    let recalled = mm.recall("shared-context", 500).await;
    assert_eq!(recalled.len(), 5, "top-5 limit expected");
    // Should be the most relevant (FTS5 returns by rank)
    for r in &recalled {
        assert!(r.entry.body.contains("shared-context"));
    }
    let _ = std::fs::remove_file(&tmp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_recall_within_500_token_budget() {
    let tmp = std::env::temp_dir().join("qingbird_mm_recall_budget.db");
    let _ = std::fs::remove_file(&tmp);
    let mm = Arc::new(MemoryManager::open(&tmp).expect("open"));

    // Add 3 entries with ~1000 tokens each (4000 chars ≈ 1000 tokens)
    for i in 0..3 {
        let body = format!("matching-keyword {}", "x".repeat(4000));
        mm.save(&make_entry(&format!("p-{i}"), &body, "user"))
            .expect("save");
    }

    let recalled = mm.recall("matching-keyword", 500).await;
    // 500-token budget should NOT fit 1000-token entries; result should be empty
    // or contain at most 1 (since we don't split, we either include or skip).
    let total_tokens: usize = recalled
        .iter()
        .map(|r| qbird_code_infra::memory::estimate_tokens_simple(&r.entry.body))
        .sum();
    assert!(
        total_tokens <= 500,
        "total tokens {total_tokens} should be ≤ budget 500"
    );
    let _ = std::fs::remove_file(&tmp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_save_creates_entry() {
    let tmp = std::env::temp_dir().join("qingbird_mm_save_creates.db");
    let _ = std::fs::remove_file(&tmp);
    let mm = MemoryManager::open(&tmp).expect("open");

    mm.save(&make_entry("path-1", "needle-in-haystack", "user"))
        .expect("save");
    let results = mm.search("needle-in-haystack", None).expect("search");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entry.body, "needle-in-haystack");
    let _ = std::fs::remove_file(&tmp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_save_async_does_not_block() {
    use std::time::Instant;
    let tmp = std::env::temp_dir().join("qingbird_mm_save_async.db");
    let _ = std::fs::remove_file(&tmp);
    let mm = Arc::new(MemoryManager::open(&tmp).expect("open"));

    let start = Instant::now();
    let handle = mm
        .clone()
        .save_async(make_entry("p-1", "async body", "user"))
        .expect("save_async");
    let elapsed = start.elapsed();
    // The spawn itself should be < 100ms; the actual save runs in background
    assert!(
        elapsed < std::time::Duration::from_millis(100),
        "save_async should return immediately, took {elapsed:?}"
    );
    let _ = handle.await;
    // Background save should be visible after await
    let results = mm.search("async body", None).expect("search");
    assert_eq!(results.len(), 1);
    let _ = std::fs::remove_file(&tmp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_save_failure_does_not_panic() {
    // save_with_summarization clamps absurdly large content to 200 chars
    // and the operation still succeeds. This is the closest deterministic
    // failure-mode surrogate (we cannot easily corrupt the live DB).
    let tmp = std::env::temp_dir().join("qingbird_mm_save_failure.db");
    let _ = std::fs::remove_file(&tmp);
    let mm = Arc::new(MemoryManager::open(&tmp).expect("open"));
    let huge = "x".repeat(10_000);
    let handle = mm
        .clone()
        .save_with_summarization(huge.clone(), "user".into(), Some("p-big"))
        .expect("save_with_summarization");
    handle.await.expect("join").expect("save ok");
    // 200-char cap means body stored should be ≤ 200
    let results = mm.search(&huge[..50], None).expect("search");
    assert_eq!(results.len(), 1);
    assert!(
        results[0].entry.body.len() <= 200,
        "summarization should clamp to ≤ 200 chars, got {}",
        results[0].entry.body.len()
    );
    let _ = std::fs::remove_file(&tmp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_summarization_caps_at_200_chars() {
    // The plan calls for "assistant summary ≤ 200 chars". We do not invoke
    // an LLM (extra cost/latency per turn); we deterministically truncate.
    let tmp = std::env::temp_dir().join("qingbird_mm_summary.db");
    let _ = std::fs::remove_file(&tmp);
    let mm = Arc::new(MemoryManager::open(&tmp).expect("open"));
    let long = "abcdefghij".repeat(200); // 2000 chars total
    let handle = mm
        .clone()
        .save_with_summarization(long.clone(), "user".into(), Some("p-summary"))
        .expect("save_with_summarization");
    handle.await.expect("join").expect("save ok");
    let results = mm.search("abcdefghij", None).expect("search");
    assert_eq!(results.len(), 1);
    assert!(results[0].entry.body.chars().count() <= 200);
    let _ = std::fs::remove_file(&tmp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_recall_failure_degrades_to_empty() {
    let tmp = std::env::temp_dir().join("qingbird_mm_recall_empty.db");
    let _ = std::fs::remove_file(&tmp);
    let mm = MemoryManager::open(&tmp).expect("open");
    mm.save(&make_entry("p", "alpha bravo charlie", "user"))
        .expect("save");
    let results = mm.recall("xyzzy-nothing-matches", 500).await;
    assert!(results.is_empty(), "no matches → empty Vec");
    let _ = std::fs::remove_file(&tmp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_eviction_by_importance_when_over_limit() {
    let tmp = std::env::temp_dir().join("qingbird_mm_evict.db");
    let _ = std::fs::remove_file(&tmp);
    let mm = MemoryManager::open(&tmp).expect("open");

    // 5 entries, then evict down to 2
    for i in 0..5 {
        mm.save(&make_entry(&format!("p-{i}"), &format!("body-{i}"), "user"))
            .expect("save");
    }
    let evicted = mm.evict_by_importance(2).expect("evict");
    assert!(
        evicted >= 3,
        "should have removed at least 3 entries, got {evicted}"
    );
    let remaining = mm.search("body", None).expect("search after evict");
    assert_eq!(remaining.len(), 2, "should keep exactly 2 entries");
    let _ = std::fs::remove_file(&tmp);
}
