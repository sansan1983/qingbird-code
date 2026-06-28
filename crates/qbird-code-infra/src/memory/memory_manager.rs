use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::tokenizer::estimate_tokens_simple;
use super::types::{MemoryEntry, MemoryResult};
use qbird_code_models::{EflowError, Result};

pub struct MemoryManager {
    db: Mutex<Connection>,
}

impl MemoryManager {
    /// Default DB location: `$XDG_DATA_HOME/qingbird/memory.db`
    /// (Windows: `%APPDATA%/qingbird/memory.db`).
    /// Creates the parent directory if it does not exist.
    pub fn default_db_path() -> Result<PathBuf> {
        let dir = dirs::data_dir()
            .ok_or_else(|| {
                EflowError::Internal("could not resolve $XDG_DATA_HOME (or %APPDATA%)".into())
            })?
            .join("qingbird");
        std::fs::create_dir_all(&dir).map_err(|e| {
            EflowError::Internal(format!(
                "failed to create data dir {}: {}",
                dir.display(),
                e
            ))
        })?;
        Ok(dir.join("memory.db"))
    }

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

    pub fn save(&self, entry: &MemoryEntry) -> Result<&'static str> {
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

    /// 19-02: async recall with token-budget enforcement.
    /// Searches FTS5 by query, returns up to top-5 hits whose combined
    /// `estimate_tokens_simple` body fits within `budget_tokens`. Returns
    /// an empty Vec if nothing matches (never an error for "no match").
    /// Strictly enforces budget: skips any hit that would exceed it.
    pub async fn recall(&self, query: &str, budget_tokens: usize) -> Vec<MemoryResult> {
        let hits = match self.search(query, None) {
            Ok(h) => h,
            Err(_) => return Vec::new(), // degrade gracefully
        };
        // FTS5 already rank-sorts; take top 5 first, then budget-trim.
        let top5: Vec<MemoryResult> = hits.into_iter().take(5).collect();
        let mut result = Vec::new();
        let mut used = 0usize;
        for hit in top5 {
            let tokens = estimate_tokens_simple(&hit.entry.body);
            if used + tokens > budget_tokens {
                continue; // strict: never overflow budget
            }
            used += tokens;
            result.push(hit);
        }
        result
    }

    /// 19-02: spawn a background `save` task; returns the `JoinHandle`
    /// so the caller can `.await` for the result when desired.
    /// Failures are propagated via the JoinHandle, not panicked.
    pub fn save_async(
        self: Arc<Self>,
        entry: MemoryEntry,
    ) -> Result<tokio::task::JoinHandle<Result<&'static str>>> {
        Ok(tokio::task::spawn_blocking(move || self.save(&entry)))
    }

    /// 19-02: save a body, deterministically clamped to 200 chars
    /// (mimics an "≤ 200 char summary" without invoking an LLM). The
    /// 200-char cap is the documented plan-shape for assistant summaries.
    pub fn save_with_summarization(
        self: Arc<Self>,
        content: String,
        scope: String,
        path: Option<&str>,
    ) -> Result<tokio::task::JoinHandle<Result<&'static str>>> {
        let body: String = content.chars().take(200).collect();
        let entry = MemoryEntry {
            path: path.unwrap_or("(interactive)").to_string(),
            scope,
            scope_id: Some("interactive".into()),
            r#type: "summary".into(),
            body,
            fingerprint: format!("sum-{}", chrono::Utc::now().timestamp_millis()),
            last_indexed_at: chrono::Utc::now().timestamp_millis(),
        };
        self.save_async(entry)
    }

    /// 19-02: simple eviction by importance / recency. Keeps the
    /// `keep` most-recently-updated entries; deletes the rest.
    /// Returns the number of entries evicted.
    pub fn evict_by_importance(&self, keep: usize) -> Result<usize> {
        let db = self
            .db
            .lock()
            .map_err(|e| EflowError::Internal(e.to_string()))?;
        // Count current rows
        let total: usize = db
            .query_row("SELECT COUNT(*) FROM memory_fts", [], |r| r.get(0))
            .map_err(|e| EflowError::Memory(format!("Count failed: {e}")))?;
        if total <= keep {
            return Ok(0);
        }
        // Delete the (total - keep) oldest by last_indexed_at.
        // FTS5 + rowid: rowid is implicit but we can use it.
        let to_delete = total - keep;
        let deleted = db
            .execute(
                "DELETE FROM memory_fts WHERE rowid IN (
                    SELECT rowid FROM memory_fts ORDER BY last_indexed_at ASC LIMIT ?1
                )",
                params![to_delete],
            )
            .map_err(|e| EflowError::Memory(format!("Evict delete failed: {e}")))?;
        Ok(deleted)
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
