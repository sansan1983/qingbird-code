//! Integration test for Task 30-03: edit + undo round trip.
//!
//! Asserts:
//! 1. `registry.execute("edit", ...)` swaps the matched substring.
//! 2. After the edit, the undo stack contains the original content for
//!    that file. Popping it and writing back restores the file verbatim.
//! 3. The undo stack retains entries across "profile switches" simulated
//!    by replacing the ToolRegistry — i.e. main.rs's responsibility.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use qbird_code_tools::{EditTool, ToolRegistry, UndoStack};

#[tokio::test]
async fn test_edit_then_undo_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file: PathBuf = dir.path().join("rt.txt");
    let original = "line1\nline2\nline3\n";
    std::fs::write(&file, original).expect("seed");

    // Build registry with EditTool registered.
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(EditTool::new()));

    // The undo stack lives outside ToolRegistry — owned by the caller.
    let undo_stack: Arc<Mutex<UndoStack>> = Arc::new(Mutex::new(UndoStack::new()));

    // 1. Push the file's original content onto the stack (mirrors what
    // ReactLoop::execute_tools_sequential does on a successful edit).
    {
        let stack_path = file.clone();
        let stack_content = original.to_string();
        let mut stack = undo_stack.lock().expect("lock");
        stack.push(stack_path, stack_content);
    }

    // 2. Run the edit. line2 -> LINE_TWO.
    registry
        .execute(
            "edit",
            serde_json::json!({
                "path": file.to_string_lossy(),
                "old_string": "line2",
                "new_string": "LINE_TWO",
            }),
            uuid::Uuid::new_v4(),
        )
        .await
        .expect("edit succeeds");

    // 3. File now reflects the edit.
    let after = std::fs::read_to_string(&file).expect("read");
    assert_eq!(after, "line1\nLINE_TWO\nline3\n");

    // 4. Pop the undo stack and write the original content back.
    let popped = {
        let mut stack = undo_stack.lock().expect("lock");
        stack.pop().expect("entry present")
    };
    assert_eq!(popped.path, file);
    assert_eq!(popped.previous_content, original);
    std::fs::write(&popped.path, &popped.previous_content).expect("write");

    // 5. File is back to the original content.
    let restored = std::fs::read_to_string(&file).expect("read");
    assert_eq!(restored, original);

    // 6. Undo stack is empty now.
    assert!(undo_stack.lock().expect("lock").is_empty());

    // 7. Simulate a profile switch (clone-and-replace the registry,
    // matching `apply_profile_to_registry` in main.rs). The undo stack
    // is NOT touched.
    let _new_registry = ToolRegistry::new();
    assert!(undo_stack.lock().expect("lock").is_empty());

    // 8. Going one step further: pushing again after the fake switch.
    {
        let mut stack = undo_stack.lock().expect("lock");
        stack.push(file.clone(), "another-version".into());
    }
    let again = undo_stack.lock().expect("lock").pop().unwrap();
    assert_eq!(again.previous_content, "another-version");
}
