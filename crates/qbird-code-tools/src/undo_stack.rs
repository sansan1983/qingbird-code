use std::collections::VecDeque;
use std::path::PathBuf;

/// Maximum number of entries retained in the undo stack (FIFO trim).
pub const MAX_UNDO_DEPTH: usize = 20;

/// A single undoable edit: the file path and the file content **before** the
/// edit was applied. Popping the stack restores the file to this snapshot.
#[derive(Debug, Clone)]
pub struct UndoEntry {
    pub path: PathBuf,
    pub previous_content: String,
}

/// LIFO undo stack with a 20-deep ring buffer. Thread-safety is left to the
/// caller (typically wrapped in `Arc<Mutex<UndoStack>>` by `main.rs`).
#[derive(Debug, Default)]
pub struct UndoStack {
    entries: VecDeque<UndoEntry>,
}

impl UndoStack {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new entry. If pushing would exceed `MAX_UNDO_DEPTH`, the
    /// oldest entry (front of the deque) is dropped — users retain the
    /// 20 most recent edits.
    pub fn push(&mut self, path: PathBuf, previous_content: String) {
        self.entries.push_back(UndoEntry {
            path,
            previous_content,
        });
        while self.entries.len() > MAX_UNDO_DEPTH {
            self.entries.pop_front();
        }
    }

    /// Pop and return the most recent entry (LIFO).
    pub fn pop(&mut self) -> Option<UndoEntry> {
        self.entries.pop_back()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_then_pop_returns_entry() {
        let mut s = UndoStack::new();
        s.push(PathBuf::from("/tmp/a.txt"), "hello".into());
        let e = s.pop().expect("entry");
        assert_eq!(e.path, PathBuf::from("/tmp/a.txt"));
        assert_eq!(e.previous_content, "hello");
        assert!(s.is_empty());
    }

    #[test]
    fn test_pop_is_lifo() {
        let mut s = UndoStack::new();
        s.push(PathBuf::from("/tmp/a"), "A".into());
        s.push(PathBuf::from("/tmp/b"), "B".into());
        let first = s.pop().unwrap();
        let second = s.pop().unwrap();
        assert_eq!(first.path, PathBuf::from("/tmp/b"));
        assert_eq!(second.path, PathBuf::from("/tmp/a"));
        assert!(s.pop().is_none());
    }

    #[test]
    fn test_ring_trims_to_max_depth() {
        let mut s = UndoStack::new();
        for i in 0..25 {
            s.push(PathBuf::from(format!("/tmp/{i}")), format!("content-{i}"));
        }
        assert_eq!(s.len(), MAX_UNDO_DEPTH);
        // pop() returns from the back (newest). After 25 pushes with depth
        // 20, entries 0..4 are dropped. The back of the deque is entry 24.
        let newest = s.pop().unwrap();
        assert_eq!(newest.path, PathBuf::from("/tmp/24"));
    }

    #[test]
    fn test_empty_stack_reports_zero() {
        let s = UndoStack::new();
        assert_eq!(s.len(), 0);
        assert!(s.is_empty());
    }
}
