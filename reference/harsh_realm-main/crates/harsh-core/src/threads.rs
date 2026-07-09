//! Thread and oracle NPC tracking for Mythic GME.
//!
//! Ported from `src/harsh_realm/engine/threads.py`.

use uuid::Uuid;

use crate::admin::oracle_dungeon::{OracleNpcRecord, OracleThreadRecord};
use crate::db::WorldDatabase;
use crate::oracle_models::{OracleNPC, Thread};
use crate::repositories::oracle::OracleRepository;

/// Manages story/character threads and the oracle NPC cast list.
pub struct ThreadTracker<'a> {
    repo: OracleRepository<'a>,
}

fn short_id() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}

fn thread_from_record(row: OracleThreadRecord) -> Thread {
    Thread {
        id: row.id,
        r#type: row.r#type,
        title: row.title,
        status: row.status,
        progress: row.progress,
    }
}

fn npc_from_record(row: OracleNpcRecord) -> OracleNPC {
    OracleNPC {
        id: row.id,
        name: row.name,
        status: row.status,
        notes: row.notes,
        entity_id: row.entity_id,
    }
}

impl<'a> ThreadTracker<'a> {
    /// Create a thread tracker over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        ThreadTracker {
            repo: OracleRepository::new(db),
        }
    }

    /// Create a new thread.
    pub fn add_thread(&self, title: &str, thread_type: &str) -> Result<Thread, String> {
        let thread_id = short_id();
        self.repo.create_thread(&thread_id, thread_type, title)?;
        Ok(Thread {
            id: thread_id,
            r#type: thread_type.to_string(),
            title: title.to_string(),
            status: "active".to_string(),
            progress: 0,
        })
    }

    /// List threads, optionally filtered by status.
    pub fn list_threads(&self, status: Option<&str>) -> Result<Vec<Thread>, String> {
        Ok(self
            .repo
            .list_threads(status)?
            .into_iter()
            .map(thread_from_record)
            .collect())
    }

    /// Get a thread by id (or prefix match).
    pub fn get_thread(&self, thread_id: &str) -> Result<Option<Thread>, String> {
        Ok(self.repo.find_thread(thread_id)?.map(thread_from_record))
    }

    /// Mark a thread as resolved.
    pub fn resolve_thread(&self, thread_id: &str) -> Result<Option<Thread>, String> {
        let Some(thread) = self.get_thread(thread_id)? else {
            return Ok(None);
        };
        self.repo.update_thread_status(&thread.id, "resolved")?;
        Ok(Some(Thread {
            status: "resolved".to_string(),
            ..thread
        }))
    }

    /// Increment a thread's progress counter.
    pub fn advance_thread(&self, thread_id: &str, amount: i32) -> Result<Option<Thread>, String> {
        let Some(thread) = self.get_thread(thread_id)? else {
            return Ok(None);
        };
        let new_progress = thread.progress + amount;
        self.repo
            .update_thread_progress(&thread.id, new_progress as i64)?;
        Ok(Some(Thread {
            progress: new_progress,
            ..thread
        }))
    }

    /// Add an NPC to the oracle cast list.
    pub fn add_npc(
        &self,
        name: &str,
        notes: &str,
        entity_id: Option<&str>,
    ) -> Result<OracleNPC, String> {
        let npc_id = short_id();
        self.repo
            .create_oracle_npc(&npc_id, name, notes, entity_id)?;
        Ok(OracleNPC {
            id: npc_id,
            name: name.to_string(),
            status: "active".to_string(),
            notes: notes.to_string(),
            entity_id: entity_id.map(str::to_string),
        })
    }

    /// List oracle NPCs, optionally filtered by status.
    pub fn list_npcs(&self, status: Option<&str>) -> Result<Vec<OracleNPC>, String> {
        Ok(self
            .repo
            .list_oracle_npcs(status)?
            .into_iter()
            .map(npc_from_record)
            .collect())
    }

    /// Remove an NPC from the oracle cast list.
    pub fn remove_npc(&self, npc_id: &str) -> Result<bool, String> {
        let Some(row) = self.repo.find_oracle_npc(npc_id)? else {
            return Ok(false);
        };
        self.repo.delete_oracle_npc(&row.id)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_lifecycle() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let tracker = ThreadTracker::new(&db);

        let thread = tracker.add_thread("Find the relic", "story").unwrap();
        assert_eq!(thread.status, "active");
        assert_eq!(thread.progress, 0);

        let advanced = tracker.advance_thread(&thread.id, 2).unwrap().unwrap();
        assert_eq!(advanced.progress, 2);

        let resolved = tracker.resolve_thread(&thread.id).unwrap().unwrap();
        assert_eq!(resolved.status, "resolved");

        let active = tracker.list_threads(Some("active")).unwrap();
        assert!(active.iter().all(|t| t.id != thread.id));

        assert!(tracker.advance_thread("nonexistent", 1).unwrap().is_none());
    }

    #[test]
    fn oracle_npc_lifecycle() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let tracker = ThreadTracker::new(&db);

        let npc = tracker.add_npc("Mara", "the smith", Some("ent-1")).unwrap();
        assert_eq!(npc.name, "Mara");
        assert_eq!(npc.entity_id.as_deref(), Some("ent-1"));

        let listed = tracker.list_npcs(None).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].notes, "the smith");

        assert!(tracker.remove_npc(&npc.id).unwrap());
        assert!(tracker.list_npcs(None).unwrap().is_empty());
        assert!(!tracker.remove_npc("ghost").unwrap());
    }
}
