//! Repository for the durable `quest_instances` table.

use crate::db::{Row, WorldDatabase};
use crate::runtime::JsonObject;

use super::models::{ActiveQuest, STATUS_COMPLETED};

/// Owns SQL access for the `quest_instances` table.
pub struct QuestRepository<'a> {
    db: &'a WorldDatabase,
}

impl<'a> QuestRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        QuestRepository { db }
    }

    /// Insert a newly-accepted quest instance.
    pub fn insert(
        &self,
        entity_id: &str,
        quest_id: &str,
        status: &str,
        accepted_tick: i32,
        progress: &JsonObject,
    ) -> Result<ActiveQuest, String> {
        let progress_json = serde_json::to_string(progress).map_err(|e| e.to_string())?;
        self.db.execute(
            "INSERT INTO quest_instances (\
                entity_id, quest_id, status, accepted_tick, progress_json\
             ) VALUES (?, ?, ?, ?, ?)",
            &[
                &entity_id,
                &quest_id,
                &status,
                &(accepted_tick as i64),
                &progress_json,
            ],
        )?;
        Ok(ActiveQuest {
            id: self.db.last_insert_rowid(),
            entity_id: entity_id.to_string(),
            quest_id: quest_id.to_string(),
            status: status.to_string(),
            accepted_tick,
            completed_tick: None,
            progress: progress.clone(),
        })
    }

    /// The non-terminal instance of `quest_id` for `entity_id`, if any.
    pub fn find_active(
        &self,
        entity_id: &str,
        quest_id: &str,
    ) -> Result<Option<ActiveQuest>, String> {
        let row = self.db.fetch_one(
            "SELECT * FROM quest_instances \
             WHERE entity_id = ? AND quest_id = ? AND status IN ('accepted', 'in_progress') \
             ORDER BY id DESC LIMIT 1",
            &[&entity_id, &quest_id],
        )?;
        row.as_ref().map(row_to_quest).transpose()
    }

    /// All active (accepted / in-progress) instances for an entity.
    pub fn list_active(&self, entity_id: &str) -> Result<Vec<ActiveQuest>, String> {
        let rows = self.db.fetch_all(
            "SELECT * FROM quest_instances \
             WHERE entity_id = ? AND status IN ('accepted', 'in_progress') ORDER BY id",
            &[&entity_id],
        )?;
        rows.iter().map(row_to_quest).collect()
    }

    /// All completed instances for an entity.
    pub fn list_completed(&self, entity_id: &str) -> Result<Vec<ActiveQuest>, String> {
        let rows = self.db.fetch_all(
            "SELECT * FROM quest_instances WHERE entity_id = ? AND status = ? ORDER BY id",
            &[&entity_id, &STATUS_COMPLETED],
        )?;
        rows.iter().map(row_to_quest).collect()
    }

    /// Every instance for an entity, any status.
    pub fn list_all(&self, entity_id: &str) -> Result<Vec<ActiveQuest>, String> {
        let rows = self.db.fetch_all(
            "SELECT * FROM quest_instances WHERE entity_id = ? ORDER BY id",
            &[&entity_id],
        )?;
        rows.iter().map(row_to_quest).collect()
    }

    /// Set the status of an instance, stamping `completed_tick` on terminal states.
    pub fn set_status(
        &self,
        id: i64,
        status: &str,
        completed_tick: Option<i32>,
    ) -> Result<(), String> {
        let completed = completed_tick.map(|v| v as i64);
        self.db.execute(
            "UPDATE quest_instances SET status = ?, completed_tick = ? WHERE id = ?",
            &[&status, &completed, &id],
        )?;
        Ok(())
    }

    /// Persist an instance's progress map.
    pub fn set_progress(&self, id: i64, progress: &JsonObject) -> Result<(), String> {
        let progress_json = serde_json::to_string(progress).map_err(|e| e.to_string())?;
        self.db.execute(
            "UPDATE quest_instances SET progress_json = ? WHERE id = ?",
            &[&progress_json, &id],
        )?;
        Ok(())
    }
}

fn row_to_quest(row: &Row) -> Result<ActiveQuest, String> {
    let progress_str = row
        .get("progress_json")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("{}");
    let progress: JsonObject = serde_json::from_str(progress_str).map_err(|e| e.to_string())?;
    Ok(ActiveQuest {
        id: row.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
        entity_id: row
            .get("entity_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        quest_id: row
            .get("quest_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        status: row
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        accepted_tick: row.get("accepted_tick").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        completed_tick: row
            .get("completed_tick")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        progress,
    })
}
