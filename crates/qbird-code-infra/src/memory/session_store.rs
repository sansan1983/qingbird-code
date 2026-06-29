use std::fs;
use std::io::Write;
use std::path::Path;

use qbird_code_models::{EflowError, Message, Result};
use rusqlite::{Connection, params};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct SessionMeta {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: i64,
}

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
            );",
        )
        .map_err(|e| EflowError::Memory(format!("Failed to create session tables: {}", e)))?;
        Ok(Self {
            db: Mutex::new(conn),
        })
    }

    #[allow(clippy::type_complexity)]
    pub fn list_sessions(&self) -> Result<Vec<(String, String, i64, i64, i64)>> {
        let db = self
            .db
            .lock()
            .map_err(|e| EflowError::Internal(e.to_string()))?;
        let mut stmt = db.prepare("SELECT id, name, created_at, updated_at, message_count FROM sessions ORDER BY updated_at DESC LIMIT 20")
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| EflowError::Memory(e.to_string()))?);
        }
        Ok(result)
    }

    pub fn save_messages(&self, session_id: &str, messages: &[Message]) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|e| EflowError::Internal(e.to_string()))?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        // Upsert session
        db.execute(
            "INSERT INTO sessions (id, name, created_at, updated_at, message_count) VALUES (?1, '', ?2, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET updated_at = ?2, message_count = ?3",
            params![session_id, now, messages.len() as i64],
        ).map_err(|e| EflowError::Memory(e.to_string()))?;

        // Clear old messages for this session and re-insert
        db.execute(
            "DELETE FROM session_messages WHERE session_id = ?1",
            params![session_id],
        )
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
        let db = self
            .db
            .lock()
            .map_err(|e| EflowError::Internal(e.to_string()))?;
        let mut stmt = db
            .prepare(
                "SELECT role, content FROM session_messages WHERE session_id = ?1 ORDER BY id ASC",
            )
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                let role: String = row.get(0)?;
                let content: String = row.get(1)?;
                Ok(match role.as_str() {
                    "system" => Message::system(content),
                    "user" => Message::user(content),
                    _ => Message::assistant(content, None),
                })
            })
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(row.map_err(|e| EflowError::Memory(e.to_string()))?);
        }
        Ok(messages)
    }

    /// Resolve `id_or_prefix` to a unique full session ID.
    ///
    /// - Exact match wins immediately.
    /// - Otherwise look for IDs starting with `id_or_prefix`.
    /// - Zero matches → `Err(SessionNotFound)`.
    /// - 2+ matches → `Err(SessionAmbiguous { count })`.
    fn resolve_id(&self, id_or_prefix: &str) -> Result<String> {
        let db = self
            .db
            .lock()
            .map_err(|e| EflowError::Internal(e.to_string()))?;

        let mut stmt = db
            .prepare("SELECT id FROM sessions WHERE id = ?1")
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        let exact: Option<String> = stmt.query_row(params![id_or_prefix], |row| row.get(0)).ok();
        if let Some(id) = exact {
            return Ok(id);
        }

        let like_pattern = format!("{}%", id_or_prefix);
        let mut stmt = db
            .prepare("SELECT id FROM sessions WHERE id LIKE ?1")
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        let mut rows = stmt
            .query(params![like_pattern])
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        let mut ids = Vec::new();
        while let Some(row) = rows.next().map_err(|e| EflowError::Memory(e.to_string()))? {
            ids.push(
                row.get::<_, String>(0)
                    .map_err(|e| EflowError::Memory(e.to_string()))?,
            );
        }
        match ids.len() {
            0 => Err(EflowError::SessionNotFound {
                id: id_or_prefix.to_string(),
            }),
            1 => Ok(ids.remove(0)),
            n => Err(EflowError::SessionAmbiguous {
                prefix: id_or_prefix.to_string(),
                count: n,
            }),
        }
    }

    /// Delete a session by ID (or unique prefix). Before deletion, writes the
    /// full message history to `<archive_dir>/<id>.jsonl` (one line per
    /// message, JSON: `{"role":"...","content":"...","timestamp":<ms>}`).
    /// Returns `Ok(())` if deleted, `Err(SessionNotFound)` if no match,
    /// or `Err(SessionAmbiguous)` if the prefix matches multiple sessions.
    pub fn delete(&self, id_or_prefix: &str, archive_dir: &Path) -> Result<()> {
        let id = self.resolve_id(id_or_prefix)?;

        let messages = self.load_messages(&id)?;

        fs::create_dir_all(archive_dir).map_err(EflowError::Io)?;
        let archive_path = archive_dir.join(format!("{}.jsonl", id));
        let mut file = fs::File::create(&archive_path).map_err(EflowError::Io)?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        for msg in &messages {
            let line = serde_json::json!({
                "role": msg.role_str(),
                "content": msg.content,
                "timestamp": now,
            })
            .to_string();
            writeln!(file, "{}", line).map_err(EflowError::Io)?;
        }

        let db = self
            .db
            .lock()
            .map_err(|e| EflowError::Internal(e.to_string()))?;
        db.execute(
            "DELETE FROM session_messages WHERE session_id = ?1",
            params![id],
        )
        .map_err(|e| EflowError::Memory(e.to_string()))?;
        db.execute("DELETE FROM sessions WHERE id = ?1", params![id])
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        Ok(())
    }

    /// Rename a session. Empty `new_name` clears the name (sets it back to '').
    pub fn rename(&self, id: &str, new_name: &str) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|e| EflowError::Internal(e.to_string()))?;
        let affected = db
            .execute(
                "UPDATE sessions SET name = ?1 WHERE id = ?2",
                params![new_name, id],
            )
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        if affected == 0 {
            Err(EflowError::SessionNotFound { id: id.to_string() })
        } else {
            Ok(())
        }
    }

    /// All sessions (no LIMIT 20) as `SessionMeta`, sorted by `updated_at DESC`.
    pub fn list_with_meta(&self) -> Result<Vec<SessionMeta>> {
        let db = self
            .db
            .lock()
            .map_err(|e| EflowError::Internal(e.to_string()))?;
        let mut stmt = db
            .prepare(
                "SELECT id, name, created_at, updated_at, message_count FROM sessions \
                 ORDER BY updated_at DESC",
            )
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SessionMeta {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    message_count: row.get(4)?,
                })
            })
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| EflowError::Memory(e.to_string()))?);
        }
        Ok(out)
    }

    /// Delete oldest-by-`updated_at` sessions until at most `keep` remain.
    /// Returns the IDs of the deleted sessions. Does **not** archive —
    /// the caller decides whether to archive beforehand.
    pub fn cleanup_old_sessions(&self, keep: usize) -> Result<Vec<String>> {
        let db = self
            .db
            .lock()
            .map_err(|e| EflowError::Internal(e.to_string()))?;
        let total: i64 = db
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        if (total as usize) <= keep {
            return Ok(Vec::new());
        }
        let to_delete = (total as usize) - keep;

        let mut stmt = db
            .prepare("SELECT id FROM sessions ORDER BY updated_at ASC LIMIT ?1")
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        let rows = stmt
            .query_map(params![to_delete as i64], |row| row.get::<_, String>(0))
            .map_err(|e| EflowError::Memory(e.to_string()))?;
        let mut ids = Vec::with_capacity(to_delete);
        for row in rows {
            ids.push(row.map_err(|e| EflowError::Memory(e.to_string()))?);
        }

        for id in &ids {
            db.execute(
                "DELETE FROM session_messages WHERE session_id = ?1",
                params![id],
            )
            .map_err(|e| EflowError::Memory(e.to_string()))?;
            db.execute("DELETE FROM sessions WHERE id = ?1", params![id])
                .map_err(|e| EflowError::Memory(e.to_string()))?;
        }
        Ok(ids)
    }
}
