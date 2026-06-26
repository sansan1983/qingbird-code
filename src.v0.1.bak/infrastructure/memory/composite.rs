use std::path::Path;

use super::manager::{MemoryEntry, MemoryManager, RecallScope};
use super::project::ProjectMemory;
use super::user::UserMemory;
use super::working::WorkingMemory;
use crate::common::error::Result;
use crate::common::types::Importance;

pub struct CompositeMemory {
    pub working: WorkingMemory,
    pub project: ProjectMemory,
    pub user: UserMemory,
}

impl CompositeMemory {
    pub fn new(working_limit: usize, project_db: &Path, user_db: &Path) -> Result<Self> {
        Ok(Self {
            working: WorkingMemory::new(working_limit),
            project: ProjectMemory::new(project_db)?,
            user: UserMemory::new(user_db)?,
        })
    }

    pub fn in_memory(working_limit: usize) -> Result<Self> {
        Ok(Self {
            working: WorkingMemory::new(working_limit),
            project: ProjectMemory::in_memory()?,
            user: UserMemory::in_memory()?,
        })
    }

    pub fn remember_smart(&mut self, entry: MemoryEntry) -> Result<uuid::Uuid> {
        if entry.importance == Importance::Low {
            self.working.remember(entry)
        } else {
            let entry_clone = entry.clone();
            let id = self.working.remember(entry)?;
            self.project.remember(entry_clone)?;
            Ok(id)
        }
    }

    pub fn recall_smart(&self, query: &str, limit: u8) -> Result<Vec<MemoryEntry>> {
        let mut results = self.working.recall(query, RecallScope::Working, limit)?;
        if results.len() >= limit as usize {
            return Ok(results);
        }

        let remaining = limit - results.len() as u8;
        if remaining > 0 {
            let project_results = self
                .project
                .recall(query, RecallScope::Project, remaining)?;
            results.extend(project_results);
        }

        if results.len() >= limit as usize {
            return Ok(results);
        }

        let remaining = limit - results.len() as u8;
        if remaining > 0 {
            let user_results = self.user.recall(query, RecallScope::User, remaining)?;
            results.extend(user_results);
        }

        Ok(results)
    }
}
