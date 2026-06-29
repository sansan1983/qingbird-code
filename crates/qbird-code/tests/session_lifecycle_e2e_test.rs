use qbird_code_infra::memory::SessionStore;
use qbird_code_models::Message;

#[test]
fn test_session_lifecycle_end_to_end_with_archive() {
    let tmp = std::env::temp_dir().join("qbird_e2e_session_lifecycle.db");
    let archive_dir = std::env::temp_dir().join("qbird_e2e_session_lifecycle.archive");
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_dir_all(&archive_dir);

    let store = SessionStore::open(&tmp).expect("open store");

    for i in 0..60 {
        let id = format!("sess-{:03}", i);
        store
            .save_messages(
                &id,
                &[
                    Message::user(format!("prompt-{}", i)),
                    Message::assistant(format!("reply-{}", i), None),
                ],
            )
            .expect("save");
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    assert_eq!(store.list_with_meta().expect("list").len(), 60);

    let meta = store.list_with_meta().expect("list for oldest");
    let oldest_ids: Vec<String> = meta.iter().rev().take(10).map(|m| m.id.clone()).collect();
    let oldest_set: std::collections::HashSet<String> = oldest_ids.iter().cloned().collect();
    assert_eq!(oldest_ids.len(), 10, "should pick 10 oldest");
    assert_eq!(oldest_set.len(), 10, "10 distinct oldest ids");

    for id in &oldest_ids {
        store.delete(id, &archive_dir).expect("delete with archive");
    }

    assert_eq!(
        store.list_with_meta().expect("list after delete").len(),
        50,
        "50 sessions should remain after deleting 10 oldest"
    );

    let mut archive_files: Vec<std::path::PathBuf> = std::fs::read_dir(&archive_dir)
        .expect("read archive dir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x == "jsonl")
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();
    archive_files.sort();
    assert_eq!(
        archive_files.len(),
        10,
        "expected exactly 10 .jsonl files in archive_dir"
    );

    let archived_ids: std::collections::HashSet<String> = archive_files
        .iter()
        .map(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .expect("file stem")
        })
        .collect();
    for id in &oldest_ids {
        assert!(
            archived_ids.contains(id),
            "archive file missing for deleted id {}",
            id
        );
    }

    let first = &archive_files[0];
    let content = std::fs::read_to_string(first).expect("read first archive");
    let lines: Vec<&str> = content.lines().collect();
    assert!(
        !lines.is_empty(),
        "archive {} should not be empty",
        first.display()
    );
    for line in &lines {
        let parsed: serde_json::Value =
            serde_json::from_str(line).expect("each line must be valid JSON");
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
        assert!(
            parsed["content"].is_string() && !parsed["content"].as_str().unwrap_or("").is_empty(),
            "content must be non-empty string: {}",
            line
        );
    }

    let extras = store
        .cleanup_old_sessions(50)
        .expect("cleanup at exactly 50");
    assert!(
        extras.is_empty(),
        "no further deletes expected when count == keep"
    );

    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_dir_all(&archive_dir);
}
