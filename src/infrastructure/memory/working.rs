use std::time::SystemTime;

use indexmap::IndexMap;
use uuid::Uuid;

use super::manager::{MemoryEntry, MemoryManager, RecallScope};
use crate::common::error::Result;
use crate::common::types::Importance;

pub struct WorkingMemory {
    entries: IndexMap<Uuid, MemoryEntry>,
    max_entries: usize,
}

impl WorkingMemory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: IndexMap::new(),
            max_entries,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl MemoryManager for WorkingMemory {
    fn remember(&mut self, mut entry: MemoryEntry) -> Result<Uuid> {
        if entry.id.is_nil() {
            entry.id = Uuid::new_v4();
        }
        let now = SystemTime::now();
        entry.created_at = now;
        entry.last_accessed_at = now;

        let id = entry.id;
        self.entries.insert(id, entry);

        if self.entries.len() > self.max_entries {
            let evict: Vec<Uuid> = self
                .entries
                .iter()
                .filter(|(_, e)| e.importance == Importance::Low)
                .map(|(id, _)| *id)
                .take(self.entries.len() - self.max_entries)
                .collect();

            for id in evict {
                self.entries.shift_remove(&id);
            }

            while self.entries.len() > self.max_entries {
                self.entries.shift_remove_index(0);
            }
        }

        Ok(id)
    }

    fn recall(&self, query: &str, _scope: RecallScope, limit: u8) -> Result<Vec<MemoryEntry>> {
        let query_lower = query.to_lowercase();
        let results: Vec<MemoryEntry> = self
            .entries
            .values()
            .rev()
            .filter(|e| {
                e.content.to_lowercase().contains(&query_lower)
                    || e.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(results)
    }

    fn recall_since(&self, since: SystemTime, _scope: RecallScope) -> Result<Vec<MemoryEntry>> {
        let results: Vec<MemoryEntry> = self
            .entries
            .values()
            .rev()
            .filter(|e| e.created_at >= since)
            .cloned()
            .collect();
        Ok(results)
    }

    fn forget(&mut self, id: Uuid) -> Result<()> {
        self.entries.shift_remove(&id);
        Ok(())
    }

    fn cleanup(&mut self) -> Result<u32> {
        let before = self.entries.len();
        let now = SystemTime::now();
        self.entries.retain(|_, e| {
            if let Some(ttl) = e.ttl
                && e.importance == Importance::Low
            {
                return now.duration_since(e.created_at).unwrap_or_default() < ttl;
            }
            true
        });
        Ok((before - self.entries.len()) as u32)
    }

    fn session_summary(&self) -> Result<String> {
        let entries: Vec<String> = self
            .entries
            .values()
            .rev()
            .take(20)
            .map(|e| {
                let preview: String = e.content.chars().take(200).collect();
                format!("- {}", preview)
            })
            .collect();
        Ok(entries.join("\n"))
    }
}
