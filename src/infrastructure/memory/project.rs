use std::path::Path;
use std::sync::Mutex;
use std::time::SystemTime;

use rusqlite::{Connection, Row, params};
use uuid::Uuid;

use super::manager::{MemoryEntry, MemoryManager, RecallScope};
use crate::common::error::{EflowError, Result};
use crate::common::types::{Importance, MemoryCategory};
use rust_i18n::t;

pub struct ProjectMemory {
    conn: Mutex<Connection>,
}

impl ProjectMemory {
    pub fn new(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                EflowError::Memory(
                    t!("err_memory_op", op = "create dir", msg = e.to_string()).to_string(),
                )
            })?;
        }

        let conn = Connection::open(db_path).map_err(|e| {
            EflowError::Memory(t!("err_memory_op", op = "open db", msg = e.to_string()).to_string())
        })?;

        let mem = Self {
            conn: Mutex::new(conn),
        };
        mem.init_schema()?;
        Ok(mem)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| {
            EflowError::Memory(
                t!("err_memory_op", op = "open in-memory", msg = e.to_string()).to_string(),
            )
        })?;
        let mem = Self {
            conn: Mutex::new(conn),
        };
        mem.init_schema()?;
        Ok(mem)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            EflowError::Memory(t!("err_memory_op", op = "lock", msg = e.to_string()).to_string())
        })?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                raw_content TEXT,
                category TEXT NOT NULL,
                importance TEXT NOT NULL DEFAULT 'Normal',
                created_at INTEGER NOT NULL,
                last_accessed_at INTEGER NOT NULL,
                ttl_secs INTEGER,
                tags TEXT NOT NULL DEFAULT '[]'
            );
            CREATE INDEX IF NOT EXISTS idx_memories_created
                ON memories(created_at);
            CREATE INDEX IF NOT EXISTS idx_memories_category
                ON memories(category);
            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                content, tags, content='memories', content_rowid='rowid'
            );
            CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
                INSERT INTO memories_fts(rowid, content, tags)
                VALUES (new.rowid, new.content, new.tags);
            END;
            CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, content, tags)
                VALUES ('delete', old.rowid, old.content, old.tags);
            END;
            CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, content, tags)
                VALUES ('delete', old.rowid, old.content, old.tags);
                INSERT INTO memories_fts(rowid, content, tags)
                VALUES (new.rowid, new.content, new.tags);
            END;",
        )
        .map_err(|e| {
            EflowError::Memory(
                t!("err_memory_op", op = "init schema", msg = e.to_string()).to_string(),
            )
        })?;
        Ok(())
    }
}

impl MemoryManager for ProjectMemory {
    fn remember(&mut self, mut entry: MemoryEntry) -> Result<Uuid> {
        if entry.id.is_nil() {
            entry.id = Uuid::new_v4();
        }
        let now = SystemTime::now();
        entry.created_at = now;
        entry.last_accessed_at = now;

        let tags_json = serde_json::to_string(&entry.tags).unwrap_or_default();
        let now_ms = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        self.conn
            .lock()
            .map_err(|e| {
                EflowError::Memory(
                    t!("err_memory_op", op = "lock", msg = e.to_string()).to_string(),
                )
            })?
            .execute(
                "INSERT OR REPLACE INTO memories (id, content, raw_content, category, importance,
                 created_at, last_accessed_at, ttl_secs, tags)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    entry.id.to_string(),
                    entry.content,
                    entry.raw_content,
                    format!("{:?}", entry.category),
                    format!("{:?}", entry.importance),
                    now_ms,
                    now_ms,
                    entry.ttl.map(|d| d.as_millis() as i64),
                    tags_json,
                ],
            )
            .map_err(|e| {
                EflowError::Memory(
                    t!("err_memory_op", op = "insert", msg = e.to_string()).to_string(),
                )
            })?;

        Ok(entry.id)
    }

    fn recall(&self, query: &str, _scope: RecallScope, limit: u8) -> Result<Vec<MemoryEntry>> {
        let conn = self.conn.lock().map_err(|e| {
            EflowError::Memory(t!("err_memory_op", op = "lock", msg = e.to_string()).to_string())
        })?;
        let mut stmt = conn
            .prepare(
                "SELECT m.id, m.content, m.raw_content, m.category, m.importance,
                    m.created_at, m.last_accessed_at, m.ttl_secs, m.tags
             FROM memories m
             INNER JOIN memories_fts fts ON m.rowid = fts.rowid
             WHERE memories_fts MATCH ?1
             ORDER BY m.last_accessed_at DESC
             LIMIT ?2",
            )
            .map_err(|e| {
                EflowError::Memory(
                    t!("err_memory_op", op = "prepare", msg = e.to_string()).to_string(),
                )
            })?;

        let rows = stmt
            .query_map(params![query, limit as i64], row_to_entry)
            .map_err(|e| {
                EflowError::Memory(
                    t!("err_memory_op", op = "query", msg = e.to_string()).to_string(),
                )
            })?;

        let mut entries = vec![];
        for entry in rows.flatten() {
            entries.push(entry);
        }
        Ok(entries)
    }

    fn recall_since(&self, since: SystemTime, _scope: RecallScope) -> Result<Vec<MemoryEntry>> {
        let since_ms = since
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let conn = self.conn.lock().map_err(|e| {
            EflowError::Memory(t!("err_memory_op", op = "lock", msg = e.to_string()).to_string())
        })?;
        let mut stmt = conn
            .prepare(
                "SELECT id, content, raw_content, category, importance,
                    created_at, last_accessed_at, ttl_secs, tags
             FROM memories WHERE created_at >= ?1
             ORDER BY created_at DESC",
            )
            .map_err(|e| {
                EflowError::Memory(
                    t!("err_memory_op", op = "prepare", msg = e.to_string()).to_string(),
                )
            })?;

        let rows = stmt
            .query_map(params![since_ms], row_to_entry)
            .map_err(|e| {
                EflowError::Memory(
                    t!("err_memory_op", op = "query", msg = e.to_string()).to_string(),
                )
            })?;

        let mut entries = vec![];
        for entry in rows.flatten() {
            entries.push(entry);
        }
        Ok(entries)
    }

    fn forget(&mut self, id: Uuid) -> Result<()> {
        self.conn
            .lock()
            .map_err(|e| {
                EflowError::Memory(
                    t!("err_memory_op", op = "lock", msg = e.to_string()).to_string(),
                )
            })?
            .execute(
                "DELETE FROM memories WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| {
                EflowError::Memory(
                    t!("err_memory_op", op = "delete", msg = e.to_string()).to_string(),
                )
            })?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<u32> {
        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let count = self
            .conn
            .lock()
            .map_err(|e| {
                EflowError::Memory(
                    t!("err_memory_op", op = "lock", msg = e.to_string()).to_string(),
                )
            })?
            .execute(
                "DELETE FROM memories WHERE
                importance = 'Low'
                AND ttl_secs IS NOT NULL
                AND (created_at + ttl_secs) < ?1",
                params![now_ms],
            )
            .map_err(|e| {
                EflowError::Memory(
                    t!("err_memory_op", op = "cleanup", msg = e.to_string()).to_string(),
                )
            })?;

        Ok(count as u32)
    }

    fn session_summary(&self) -> Result<String> {
        let conn = self.conn.lock().map_err(|e| {
            EflowError::Memory(t!("err_memory_op", op = "lock", msg = e.to_string()).to_string())
        })?;
        let mut stmt = conn
            .prepare("SELECT content FROM memories ORDER BY last_accessed_at DESC LIMIT 20")
            .map_err(|e| {
                EflowError::Memory(
                    t!("err_memory_op", op = "prepare", msg = e.to_string()).to_string(),
                )
            })?;

        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| {
                EflowError::Memory(
                    t!("err_memory_op", op = "query", msg = e.to_string()).to_string(),
                )
            })?;

        let entries: Vec<String> = rows
            .filter_map(|r| r.ok())
            .map(|c| {
                let preview: String = c.chars().take(200).collect();
                format!("- {}", preview)
            })
            .collect();

        Ok(entries.join("\n"))
    }
}

fn row_to_entry(row: &Row) -> rusqlite::Result<MemoryEntry> {
    let id_str: String = row.get(0)?;
    let content: String = row.get(1)?;
    let raw_content: Option<String> = row.get(2)?;
    let category_str: String = row.get(3)?;
    let importance_str: String = row.get(4)?;
    let created_ms: i64 = row.get(5)?;
    let accessed_ms: i64 = row.get(6)?;
    let ttl_ms: Option<i64> = row.get(7)?;
    let tags_json: String = row.get(8)?;

    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

    Ok(MemoryEntry {
        id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()),
        content,
        raw_content,
        category: parse_category(&category_str),
        importance: parse_importance(&importance_str),
        created_at: SystemTime::UNIX_EPOCH
            + std::time::Duration::from_millis(created_ms.max(0) as u64),
        last_accessed_at: SystemTime::UNIX_EPOCH
            + std::time::Duration::from_millis(accessed_ms.max(0) as u64),
        ttl: ttl_ms.map(|s| std::time::Duration::from_millis(s.max(0) as u64)),
        tags,
    })
}

fn parse_category(s: &str) -> MemoryCategory {
    match s {
        "TaskResult" => MemoryCategory::TaskResult,
        "Decision" => MemoryCategory::Decision,
        "Feedback" => MemoryCategory::Feedback,
        "UserPreference" => MemoryCategory::UserPreference,
        "LearnedPattern" => MemoryCategory::LearnedPattern,
        "ManualNote" => MemoryCategory::ManualNote,
        _ => MemoryCategory::TaskResult,
    }
}

fn parse_importance(s: &str) -> Importance {
    match s {
        "Low" => Importance::Low,
        "High" => Importance::High,
        "Pinned" => Importance::Pinned,
        _ => Importance::Normal,
    }
}
