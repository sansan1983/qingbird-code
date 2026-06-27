use qbird_code_models::{EflowError, Message, Result};
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;

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
}
