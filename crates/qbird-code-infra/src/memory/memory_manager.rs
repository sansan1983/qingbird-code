use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;

use super::types::{MemoryEntry, MemoryResult};
use qbird_code_models::{EflowError, Result};

pub struct MemoryManager {
    db: Mutex<Connection>,
}

impl MemoryManager {
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .map_err(|e| EflowError::Memory(format!("Failed to open DB: {}", e)))?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| EflowError::Memory(format!("Failed to set WAL: {}", e)))?;

        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
                path UNINDEXED,
                scope UNINDEXED,
                scope_id UNINDEXED,
                type UNINDEXED,
                body,
                fingerprint UNINDEXED,
                last_indexed_at UNINDEXED,
                tokenize='trigram'
            );",
        )
        .map_err(|e| EflowError::Memory(format!("Failed to create FTS5 table: {}", e)))?;

        Ok(Self {
            db: Mutex::new(conn),
        })
    }

    pub fn save(&self, entry: &MemoryEntry) -> Result<&str> {
        let db = self
            .db
            .lock()
            .map_err(|e| EflowError::Internal(e.to_string()))?;

        let existing: std::result::Result<String, _> = db.query_row(
            "SELECT fingerprint FROM memory_fts WHERE path = ?1",
            params![entry.path],
            |row| row.get(0),
        );

        match existing {
            Ok(fp) if fp == entry.fingerprint => return Ok("unchanged"),
            Ok(_) => {
                db.execute(
                    "DELETE FROM memory_fts WHERE path = ?1",
                    params![entry.path],
                )
                .map_err(|e| EflowError::Memory(format!("Delete failed: {}", e)))?;
            }
            Err(_) => {}
        }

        db.execute(
            "INSERT INTO memory_fts (path, scope, scope_id, type, body, fingerprint, last_indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.path,
                entry.scope,
                entry.scope_id.as_deref().unwrap_or(""),
                entry.r#type,
                entry.body,
                entry.fingerprint,
                entry.last_indexed_at,
            ],
        )
        .map_err(|e| EflowError::Memory(format!("Insert failed: {}", e)))?;

        Ok("created")
    }

    pub fn search(&self, query: &str, scope: Option<&[String]>) -> Result<Vec<MemoryResult>> {
        let db = self
            .db
            .lock()
            .map_err(|e| EflowError::Internal(e.to_string()))?;

        let tokens: Vec<&str> = query.split_whitespace().filter(|t| t.len() > 1).collect();
        if tokens.is_empty() {
            return Ok(vec![]);
        }
        let fts_query = tokens
            .iter()
            .map(|t| format!("\"{}\"", t))
            .collect::<Vec<_>>()
            .join(" OR ");

        let mut sql = String::from(
            "SELECT path, scope, scope_id, type, body, fingerprint FROM memory_fts WHERE body MATCH ?1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(fts_query)];

        if let Some(scopes) = scope
            && !scopes.is_empty()
        {
            let scope_conditions: Vec<String> = scopes
                .iter()
                .enumerate()
                .map(|(i, _)| format!("scope = ?{}", i + 2))
                .collect();
            sql.push_str(&format!(" AND ({})", scope_conditions.join(" OR ")));
            for s in scopes {
                param_values.push(Box::new(s.clone()));
            }
        }

        sql.push_str(" LIMIT 20");

        let mut stmt = db
            .prepare(&sql)
            .map_err(|e| EflowError::Memory(format!("Prepare failed: {}", e)))?;

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(MemoryResult {
                    entry: MemoryEntry {
                        path: row.get(0)?,
                        scope: row.get(1)?,
                        scope_id: row.get(2)?,
                        r#type: row.get(3)?,
                        body: row.get(4)?,
                        fingerprint: row.get(5)?,
                        last_indexed_at: 0,
                    },
                    score: 1.0,
                })
            })
            .map_err(|e| EflowError::Memory(format!("Query failed: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| EflowError::Memory(format!("Row failed: {}", e)))?);
        }
        Ok(results)
    }
}
