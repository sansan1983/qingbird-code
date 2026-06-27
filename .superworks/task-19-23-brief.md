# Tasks 19-23: Daily Usability Polish (v0.2.17)

---

## Task 19: Token usage display

**Modify:** `crates/qbird-code-agents/src/react_loop/types.rs` and `mod.rs`

In `LoopState`, add:
```rust
pub total_prompt_tokens: u64,
pub total_completion_tokens: u64,
```

In `ReactLoop::run()`, after each LLM response, accumulate:
```rust
state.total_prompt_tokens += chat_response.usage.prompt_tokens;
state.total_completion_tokens += chat_response.usage.completion_tokens;
tracing::info!(
    "Token usage: prompt={}, completion={}, total={}",
    chat_response.usage.prompt_tokens,
    chat_response.usage.completion_tokens,
    chat_response.usage.prompt_tokens + chat_response.usage.completion_tokens,
);
```

In `main.rs`, add `/usage` slash command that prints accumulated usage from agent result.

---

## Task 20: Session persistence

**Create:** `crates/qbird-code-infra/src/memory/session_store.rs`
**Modify:** `crates/qbird-code-infra/src/memory/mod.rs`
**Modify:** `crates/qbird-code/src/main.rs`

SessionStore uses SQLite to persist conversation history:

```rust
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;
use qbird_code_models::{EflowError, Result, Message};

pub struct SessionStore {
    db: Mutex<Connection>,
}

impl SessionStore {
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .map_err(|e| EflowError::Memory(format!("Failed to open session DB: {}", e)))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT DEFAULT '',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                message_count INTEGER DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS session_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );"
        ).map_err(|e| EflowError::Memory(format!("Failed to create session tables: {}", e)))?;
        Ok(Self { db: Mutex::new(conn) })
    }

    pub fn list_sessions(&self) -> Result<Vec<(String, String, i64, i64, i64)>> {
        let db = self.db.lock().map_err(|e| EflowError::Internal(e.to_string()))?;
        let mut stmt = db.prepare("SELECT id, name, created_at, updated_at, message_count FROM sessions ORDER BY updated_at DESC LIMIT 20")
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?))
        }).map_err(|e| EflowError::Memory(e.to_string()))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| EflowError::Memory(e.to_string()))?);
        }
        Ok(result)
    }

    pub fn save_messages(&self, session_id: &str, messages: &[Message]) -> Result<()> {
        let db = self.db.lock().map_err(|e| EflowError::Internal(e.to_string()))?;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64;

        // Upsert session
        db.execute(
            "INSERT INTO sessions (id, name, created_at, updated_at, message_count) VALUES (?1, '', ?2, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET updated_at = ?2, message_count = ?3",
            params![session_id, now, messages.len() as i64],
        ).map_err(|e| EflowError::Memory(e.to_string()))?;

        // Clear old messages for this session and re-insert
        db.execute("DELETE FROM session_messages WHERE session_id = ?1", params![session_id])
            .map_err(|e| EflowError::Memory(e.to_string()))?;

        for msg in messages {
            db.execute(
                "INSERT INTO session_messages (session_id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4)",
                params![session_id, msg.role_str(), msg.content, now],
            ).map_err(|e| EflowError::Memory(e.to_string()))?;
        }
        Ok(())
    }

    pub fn load_messages(&self, session_id: &str) -> Result<Vec<Message>> {
        let db = self.db.lock().map_err(|e| EflowError::Internal(e.to_string()))?;
        let mut stmt = db.prepare("SELECT role, content FROM session_messages WHERE session_id = ?1 ORDER BY id ASC")
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        let rows = stmt.query_map(params![session_id], |row| {
            let role: String = row.get(0)?;
            let content: String = row.get(1)?;
            Ok(match role.as_str() {
                "system" => Message::system(content),
                "user" => Message::user(content),
                _ => Message::assistant(content, None),
            })
        }).map_err(|e| EflowError::Memory(e.to_string()))?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(row.map_err(|e| EflowError::Memory(e.to_string()))?);
        }
        Ok(messages)
    }
}
```

In `main.rs`:
- Add `use qbird_code_infra::memory::SessionStore;`
- Add `/sessions` and `/session load <id>` commands
- On `--interactive` start, load or create session
- On exit, save messages

---

## Task 21: Tool enhancements

**Modify:** `crates/qbird-code-tools/src/registry.rs`

Add MAX_OUTPUT_TOKENS constant and content truncation in ToolRegistry.execute():
```rust
pub const MAX_OUTPUT_TOKENS: usize = 4000;

// After tool execution, truncate oversized content
let mut output = tool.execute(params).await?;
let estimated_tokens = output.content.len() / 3; // rough estimate
if estimated_tokens > MAX_OUTPUT_TOKENS {
    let max_chars = MAX_OUTPUT_TOKENS * 3;
    let truncated = output.content.char_indices()
        .nth(max_chars)
        .map(|(i, _)| i)
        .unwrap_or(output.content.len());
    output.content = format!("{}...[Output truncated at ~{} tokens]", 
        &output.content[..truncated], MAX_OUTPUT_TOKENS);
    if let Some(ref mut meta) = output.metadata {
        meta["truncated"] = serde_json::json!(true);
    }
}
```

---

## Task 22: Subagent pool (basic)

**Create:** `crates/qbird-code-agents/src/subagent_pool.rs`
**Modify:** `crates/qbird-code-agents/src/lib.rs`

```rust
use std::sync::Arc;
use tokio::sync::mpsc;
use qbird_code_models::EflowError;

pub struct SubagentPool {
    size: usize,
}

impl SubagentPool {
    pub fn new(size: usize) -> Self {
        Self { size: size.max(1) }
    }

    pub fn size(&self) -> usize { self.size }
}

pub async fn execute_parallel<F, T>(tasks: Vec<F>) -> Vec<Result<T, EflowError>>
where
    F: Future<Output = Result<T, EflowError>> + Send,
    T: Send + 'static,
{
    use futures_util::stream::{FuturesUnordered, StreamExt};
    let mut futures: FuturesUnordered<tokio::task::JoinHandle<Result<T, EflowError>>> = FuturesUnordered::new();
    for task in tasks {
        futures.push(tokio::spawn(task));
    }
    let mut results = Vec::new();
    while let Some(result) = futures.next().await {
        match result {
            Ok(Ok(val)) => results.push(Ok(val)),
            Ok(Err(e)) => results.push(Err(e)),
            Err(join_err) => results.push(Err(EflowError::Internal(format!("Task panicked: {}", join_err)))),
        }
    }
    results
}
```

---

## Task 23: i18n audit + version bump

1. Search codebase for `println!` and `eprintln!` — ensure all user-facing strings use `t!()`
2. Search for `tracing::info!` / `tracing::warn!` — ensure all are in English
3. Bump Cargo.toml 0.2.16 → 0.2.17
4. CHANGELOG:

```
## [0.2.17] - 2026-06-27

### Added
- Token 用量追踪和展示（/usage 命令）
- 对话历史持久化（SQLite 存储，/sessions 命令）
- 工具输出大小限制（防止撑爆上下文）
- Subagent 基础并发池

### Fixed
- 全量 i18n 审计：确保所有用户面向字符串走 t!()
```

---

## Verification

```bash
cargo build && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test
```
