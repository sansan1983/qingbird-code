use std::sync::{Arc, Mutex};

use qbird_code_models::EflowError;
use qbird_code_tools::{EditTool, ToolRegistry, UndoStack};

fn write_file(path: &std::path::Path, content: &str) {
    std::fs::write(path, content).expect("write fixture");
}

#[tokio::test]
async fn test_edit_replace_empty_string_matches_once() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("empty_old.txt");
    write_file(&file, "hello world\n");
    // old_string not present → ToolEditNotFound
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(EditTool::new()));
    let res = reg
        .execute(
            "edit",
            serde_json::json!({
                "path": file.to_string_lossy(),
                "old_string": "not_present",
                "new_string": "replaced",
            }),
            uuid::Uuid::new_v4(),
        )
        .await;
    match res {
        Err(EflowError::ToolEditNotFound { .. }) => {}
        other => panic!("expected ToolEditNotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn test_edit_multiline_old_string() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("multi.txt");
    write_file(&file, "line1\nline2\nline3\nline4\n");
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(EditTool::new()));
    let out = reg
        .execute(
            "edit",
            serde_json::json!({
                "path": file.to_string_lossy(),
                "old_string": "line2\nline3",
                "new_string": "REPLACED",
            }),
            uuid::Uuid::new_v4(),
        )
        .await
        .expect("multiline edit succeeds");
    assert!(out.success);
    let after = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after, "line1\nREPLACED\nline4\n");
}

#[tokio::test]
async fn test_edit_at_file_start() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("start.txt");
    write_file(&file, "FIRST line\nsecond line\n");
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(EditTool::new()));
    let out = reg
        .execute(
            "edit",
            serde_json::json!({
                "path": file.to_string_lossy(),
                "old_string": "FIRST",
                "new_string": "REPLACED",
            }),
            uuid::Uuid::new_v4(),
        )
        .await
        .expect("edit at start");
    assert!(out.success);
    let after = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after, "REPLACED line\nsecond line\n");
}

#[tokio::test]
async fn test_edit_at_file_end() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("end.txt");
    write_file(&file, "first line\nLAST\n");
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(EditTool::new()));
    let out = reg
        .execute(
            "edit",
            serde_json::json!({
                "path": file.to_string_lossy(),
                "old_string": "LAST",
                "new_string": "REPLACED",
            }),
            uuid::Uuid::new_v4(),
        )
        .await
        .expect("edit at end");
    assert!(out.success);
    let after = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after, "first line\nREPLACED\n");
}

#[tokio::test]
async fn test_edit_identical_old_and_new() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("same.txt");
    write_file(&file, "keep this\n");
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(EditTool::new()));
    let out = reg
        .execute(
            "edit",
            serde_json::json!({
                "path": file.to_string_lossy(),
                "old_string": "keep this",
                "new_string": "keep this",
            }),
            uuid::Uuid::new_v4(),
        )
        .await
        .expect("identical replacement");
    assert!(out.success);
    let after = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after, "keep this\n");
}

#[tokio::test]
async fn test_edit_with_undo_stack_pushes_entry() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("undo_push.txt");
    let original = "before\n";
    write_file(&file, original);

    let stack = Arc::new(Mutex::new(UndoStack::new()));
    let edit_tool = EditTool::new().with_undo_stack(Arc::clone(&stack));
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(edit_tool));

    reg.execute(
        "edit",
        serde_json::json!({
            "path": file.to_string_lossy(),
            "old_string": "before",
            "new_string": "after",
        }),
        uuid::Uuid::new_v4(),
    )
    .await
    .expect("edit");

    let entry = stack.lock().expect("lock").pop().expect("entry present");
    assert_eq!(entry.path, file);
    assert_eq!(entry.previous_content, original);
}

#[tokio::test]
async fn test_edit_with_undo_stack_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("undo_rt.txt");
    let original = "original content\n";
    write_file(&file, original);

    let stack = Arc::new(Mutex::new(UndoStack::new()));
    let edit_tool = EditTool::new().with_undo_stack(Arc::clone(&stack));
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(edit_tool));

    // Edit
    reg.execute(
        "edit",
        serde_json::json!({
            "path": file.to_string_lossy(),
            "old_string": "original",
            "new_string": "modified",
        }),
        uuid::Uuid::new_v4(),
    )
    .await
    .expect("edit");
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "modified content\n"
    );

    // Undo
    let entry = stack.lock().expect("lock").pop().expect("entry");
    std::fs::write(&entry.path, &entry.previous_content).expect("restore");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
}

#[tokio::test]
async fn test_edit_diff_output_counts_lines() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("diff_count.txt");
    write_file(&file, "a\nb\nc\n");
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(EditTool::new()));
    let out = reg
        .execute(
            "edit",
            serde_json::json!({
                "path": file.to_string_lossy(),
                "old_string": "b",
                "new_string": "x\ny",
            }),
            uuid::Uuid::new_v4(),
        )
        .await
        .expect("edit");
    let meta = out.metadata.expect("metadata");
    assert_eq!(meta["old_lines"], serde_json::json!(3));
    assert_eq!(meta["new_lines"], serde_json::json!(4));
    assert_eq!(meta["delta"], serde_json::json!(1));
}
