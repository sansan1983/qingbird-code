use std::time::SystemTime;

use uuid::Uuid;

use crate::common::types::{Importance, MemoryCategory};

#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub content: String,
    pub raw_content: Option<String>,
    pub category: MemoryCategory,
    pub importance: Importance,
    pub created_at: SystemTime,
    pub last_accessed_at: SystemTime,
    pub ttl: Option<std::time::Duration>,
    pub tags: Vec<String>,
}

impl MemoryEntry {
    pub fn new(
        content: impl Into<String>,
        category: MemoryCategory,
        importance: Importance,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            content: content.into(),
            raw_content: None,
            category,
            importance,
            created_at: SystemTime::now(),
            last_accessed_at: SystemTime::now(),
            ttl: None,
            tags: vec![],
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_ttl(mut self, ttl: std::time::Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecallScope {
    Working,
    Project,
    User,
}

pub trait MemoryManager: Send + Sync {
    fn remember(&mut self, entry: MemoryEntry) -> crate::common::error::Result<Uuid>;
    fn recall(
        &self,
        query: &str,
        scope: RecallScope,
        limit: u8,
    ) -> crate::common::error::Result<Vec<MemoryEntry>>;
    fn recall_since(
        &self,
        since: SystemTime,
        scope: RecallScope,
    ) -> crate::common::error::Result<Vec<MemoryEntry>>;
    fn forget(&mut self, id: Uuid) -> crate::common::error::Result<()>;
    fn cleanup(&mut self) -> crate::common::error::Result<u32>;
    fn session_summary(&self) -> crate::common::error::Result<String>;
}
