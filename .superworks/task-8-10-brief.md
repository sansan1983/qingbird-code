# Tasks 8-10: Memory module infrastructure

Add rusqlite dependency and create the memory module scaffolding with types, tokenizer, overflow detection, and budgeted read.

---

## Task 8: 创建 memory 模块基础设施

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `crates/qbird-code-infra/Cargo.toml`
- Create: `crates/qbird-code-infra/src/memory/mod.rs`
- Create: `crates/qbird-code-infra/src/memory/types.rs`
- Modify: `crates/qbird-code-infra/src/lib.rs`

### Step 1: 在 workspace `Cargo.toml` 添加 rusqlite

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
```

### Step 2: 在 `crates/qbird-code-infra/Cargo.toml` 添加

```toml
rusqlite = { workspace = true }
```

### Step 3: 创建 `crates/qbird-code-infra/src/memory/types.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub path: String,
    pub scope: String,
    pub scope_id: Option<String>,
    pub r#type: String,
    pub body: String,
    pub fingerprint: String,
    pub last_indexed_at: i64,
}

#[derive(Debug, Clone)]
pub struct MemoryResult {
    pub entry: MemoryEntry,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct BudgetedReadResult {
    pub text: String,
    pub truncated: bool,
    pub total_tokens: usize,
    pub used_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct ContextMessage {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct CheckpointEvent {
    pub checkpoint_id: String,
    pub token_count: usize,
    pub reason: String,
    pub session_id: String,
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub model_limit: usize,
    pub current_usage: usize,
    pub reserved: usize,
}
```

### Step 4: 创建 `crates/qbird-code-infra/src/memory/mod.rs`

```rust
pub mod types;
pub mod tokenizer;
pub mod overflow;
pub mod budgeted_read;
pub mod memory_manager;
pub mod context_manager;

pub use memory_manager::MemoryManager;
pub use context_manager::ContextManager;
pub use tokenizer::estimate_tokens_simple;
pub use overflow::OverflowLevel;
pub use types::*;
```

### Step 5: 在 `crates/qbird-code-infra/src/lib.rs` 注册

```rust
pub mod memory;
```

---

## Task 9: tokenizer + overflow

### 创建 `crates/qbird-code-infra/src/memory/tokenizer.rs`

```rust
pub fn estimate_tokens_simple(text: &str) -> usize {
    let mut chinese_chars: usize = 0;
    let mut other_chars: usize = 0;
    for ch in text.chars() {
        if ch >= '\u{4e00}' && ch <= '\u{9fff}' {
            chinese_chars += 1;
        } else {
            other_chars += 1;
        }
    }
    let chinese_tokens = (chinese_chars as f64 * 0.5).ceil() as usize;
    let other_tokens = (other_chars as f64 / 4.0).ceil() as usize;
    chinese_tokens + other_tokens
}

pub fn tokens_to_chars(tokens: usize) -> usize {
    tokens * 3
}
```

### 创建 `crates/qbird-code-infra/src/memory/overflow.rs`

```rust
use super::types::TokenInfo;

pub type OverflowLevel = u8;

pub fn usable(input: &TokenInfo) -> usize {
    let reserved = input.reserved.clamp(2000, 8000);
    input.model_limit.saturating_sub(input.current_usage).saturating_sub(reserved)
}

pub fn overflow_level(input: &TokenInfo) -> OverflowLevel {
    let available = usable(input);
    if available == 0 { return 3; }
    let ratio = input.current_usage as f64 / available as f64;
    if ratio < 0.50 { 0 }
    else if ratio < 0.70 { 1 }
    else if ratio < 0.85 { 2 }
    else { 3 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_overflow_safe() {
        let info = TokenInfo { model_limit: 64000, current_usage: 10000, reserved: 4000 };
        assert_eq!(overflow_level(&info), 0);
    }
    #[test]
    fn test_overflow_danger() {
        let info = TokenInfo { model_limit: 64000, current_usage: 55000, reserved: 4000 };
        assert_eq!(overflow_level(&info), 3);
    }
}
```

---

## Task 10: budgeted_read

### 创建 `crates/qbird-code-infra/src/memory/budgeted_read.rs`

```rust
use super::tokenizer::{estimate_tokens_simple, tokens_to_chars};
use super::types::BudgetedReadResult;

pub fn read_budgeted(text: &str, budget_tokens: usize) -> BudgetedReadResult {
    let total_tokens = estimate_tokens_simple(text);
    if total_tokens <= budget_tokens {
        return BudgetedReadResult {
            text: text.to_string(),
            truncated: false,
            total_tokens,
            used_tokens: total_tokens,
        };
    }
    let max_chars = tokens_to_chars(budget_tokens.saturating_sub(5));
    let end = text.char_indices().nth(max_chars).map(|(i, _)| i).unwrap_or(text.len());
    let truncated = &text[..end];
    let text = format!("{}\n\n_[truncated, budget exceeded]_", truncated);
    BudgetedReadResult {
        text,
        truncated: true,
        total_tokens,
        used_tokens: budget_tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_read_budgeted_within_budget() {
        let result = read_budgeted("hello world", 100);
        assert!(!result.truncated);
    }
    #[test]
    fn test_read_budgeted_exceeds() {
        let text = "A".repeat(1000);
        let result = read_budgeted(&text, 10);
        assert!(result.truncated);
    }
}
```

---

## 验证

```bash
cargo build && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test
```
