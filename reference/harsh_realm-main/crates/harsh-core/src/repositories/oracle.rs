//! Repository for oracle threads, cast NPCs, and plotlines.
//!
//! Ported from `src/harsh_realm/engine/oracle_repository.py`.

use crate::admin::oracle_dungeon::{OracleNpcRecord, OracleThreadRecord};
use crate::db::{Row, WorldDatabase};
use crate::oracle_models::PlotlineRecord;
use crate::scene_data::AdventureScene;

/// Centralizes DB access for oracle-side tracking tables.
pub struct OracleRepository<'a> {
    db: &'a WorldDatabase,
}

fn rs(row: &Row, key: &str) -> String {
    row.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}
fn ros(row: &Row, key: &str) -> Option<String> {
    row.get(key).and_then(|v| v.as_str()).map(str::to_string)
}
fn ri(row: &Row, key: &str) -> i64 {
    row.get(key).and_then(|v| v.as_i64()).unwrap_or(0)
}

fn thread_from_row(row: &Row) -> OracleThreadRecord {
    OracleThreadRecord {
        id: rs(row, "id"),
        r#type: rs(row, "type"),
        title: rs(row, "title"),
        status: rs(row, "status"),
        progress: ri(row, "progress") as i32,
    }
}

fn npc_from_row(row: &Row) -> OracleNpcRecord {
    OracleNpcRecord {
        id: rs(row, "id"),
        name: rs(row, "name"),
        status: rs(row, "status"),
        notes: rs(row, "notes"),
        entity_id: ros(row, "entity_id"),
    }
}

impl<'a> OracleRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        OracleRepository { db }
    }

    /// Insert a new thread row.
    pub fn create_thread(&self, thread_id: &str, thread_type: &str, title: &str) -> Result<(), String> {
        self.db.execute(
            "INSERT INTO threads (id, type, title, status, progress) VALUES (?, ?, ?, 'active', 0)",
            &[&thread_id, &thread_type, &title],
        )?;
        Ok(())
    }

    /// List thread rows, optionally filtered by status.
    pub fn list_threads(&self, status: Option<&str>) -> Result<Vec<OracleThreadRecord>, String> {
        let rows = match status {
            Some(s) => self.db.fetch_all(
                "SELECT id, type, title, status, progress FROM threads WHERE status = ? ORDER BY title",
                &[&s],
            )?,
            None => self.db.fetch_all(
                "SELECT id, type, title, status, progress FROM threads ORDER BY title",
                &[],
            )?,
        };
        Ok(rows.iter().map(thread_from_row).collect())
    }

    /// Find a thread by exact or unique prefix id.
    pub fn find_thread(&self, thread_id: &str) -> Result<Option<OracleThreadRecord>, String> {
        if let Some(row) = self.db.fetch_one(
            "SELECT id, type, title, status, progress FROM threads WHERE id = ?",
            &[&thread_id],
        )? {
            return Ok(Some(thread_from_row(&row)));
        }
        let like = format!("{thread_id}%");
        let rows = self.db.fetch_all(
            "SELECT id, type, title, status, progress FROM threads WHERE id LIKE ?",
            &[&like],
        )?;
        Ok(if rows.len() == 1 { Some(thread_from_row(&rows[0])) } else { None })
    }

    /// Persist a thread status change.
    pub fn update_thread_status(&self, thread_id: &str, status: &str) -> Result<(), String> {
        self.db.execute("UPDATE threads SET status = ? WHERE id = ?", &[&status, &thread_id])?;
        Ok(())
    }

    /// Persist a thread progress counter.
    pub fn update_thread_progress(&self, thread_id: &str, progress: i64) -> Result<(), String> {
        self.db.execute("UPDATE threads SET progress = ? WHERE id = ?", &[&progress, &thread_id])?;
        Ok(())
    }

    /// Insert a new oracle NPC row.
    pub fn create_oracle_npc(
        &self,
        npc_id: &str,
        name: &str,
        notes: &str,
        entity_id: Option<&str>,
    ) -> Result<(), String> {
        self.db.execute(
            "INSERT INTO oracle_npcs (id, name, status, notes, entity_id) VALUES (?, ?, 'active', ?, ?)",
            &[&npc_id, &name, &notes, &entity_id],
        )?;
        Ok(())
    }

    /// List oracle NPC rows, optionally filtered by status.
    pub fn list_oracle_npcs(&self, status: Option<&str>) -> Result<Vec<OracleNpcRecord>, String> {
        let rows = match status {
            Some(s) => self.db.fetch_all(
                "SELECT id, name, status, notes, entity_id FROM oracle_npcs WHERE status = ? ORDER BY name",
                &[&s],
            )?,
            None => self.db.fetch_all(
                "SELECT id, name, status, notes, entity_id FROM oracle_npcs ORDER BY name",
                &[],
            )?,
        };
        Ok(rows.iter().map(npc_from_row).collect())
    }

    /// Find an oracle NPC by exact or unique prefix id.
    pub fn find_oracle_npc(&self, npc_id: &str) -> Result<Option<OracleNpcRecord>, String> {
        if let Some(row) = self.db.fetch_one(
            "SELECT id, name, status, notes, entity_id FROM oracle_npcs WHERE id = ?",
            &[&npc_id],
        )? {
            return Ok(Some(npc_from_row(&row)));
        }
        let like = format!("{npc_id}%");
        let rows = self.db.fetch_all(
            "SELECT id, name, status, notes, entity_id FROM oracle_npcs WHERE id LIKE ?",
            &[&like],
        )?;
        Ok(if rows.len() == 1 { Some(npc_from_row(&rows[0])) } else { None })
    }

    /// Delete an oracle NPC row by exact id.
    pub fn delete_oracle_npc(&self, npc_id: &str) -> Result<(), String> {
        self.db.execute("DELETE FROM oracle_npcs WHERE id = ?", &[&npc_id])?;
        Ok(())
    }

    /// Insert a new plotline row.
    pub fn create_plotline(&self, plotline_id: &str, title: &str, theme: &str) -> Result<(), String> {
        self.db.execute(
            "INSERT INTO plotlines (id, title, theme, status, scenes) VALUES (?, ?, ?, 'active', '[]')",
            &[&plotline_id, &title, &theme],
        )?;
        Ok(())
    }

    /// List plotline rows, optionally filtered by status.
    pub fn list_plotlines(&self, status: Option<&str>) -> Result<Vec<PlotlineRecord>, String> {
        let rows = match status {
            Some(s) => self.db.fetch_all(
                "SELECT id, title, theme, status, scenes FROM plotlines WHERE status = ? ORDER BY title",
                &[&s],
            )?,
            None => self.db.fetch_all(
                "SELECT id, title, theme, status, scenes FROM plotlines ORDER BY title",
                &[],
            )?,
        };
        let mut out = Vec::new();
        for row in &rows {
            let id = rs(row, "id");
            out.push(PlotlineRecord {
                scenes: self.load_plotline_scenes(&id)?,
                id,
                title: rs(row, "title"),
                theme: rs(row, "theme"),
                status: rs(row, "status"),
            });
        }
        Ok(out)
    }

    /// Find a plotline by exact or unique prefix id.
    pub fn find_plotline(&self, plotline_id: &str) -> Result<Option<PlotlineRecord>, String> {
        let exact = self.db.fetch_one(
            "SELECT id, title, theme, status, scenes FROM plotlines WHERE id = ?",
            &[&plotline_id],
        )?;
        let row = match exact {
            Some(row) => Some(row),
            None => {
                let like = format!("{plotline_id}%");
                let rows = self.db.fetch_all(
                    "SELECT id, title, theme, status, scenes FROM plotlines WHERE id LIKE ?",
                    &[&like],
                )?;
                if rows.len() == 1 { Some(rows.into_iter().next().unwrap()) } else { None }
            }
        };
        let Some(row) = row else { return Ok(None) };
        let id = rs(&row, "id");
        Ok(Some(PlotlineRecord {
            scenes: self.load_plotline_scenes(&id)?,
            id,
            title: rs(&row, "title"),
            theme: rs(&row, "theme"),
            status: rs(&row, "status"),
        }))
    }

    /// Persist a plotline's scene list (legacy column + normalized rows).
    pub fn update_plotline_scenes(
        &self,
        plotline_id: &str,
        scenes: &[AdventureScene],
    ) -> Result<(), String> {
        let json = serde_json::to_string(scenes).map_err(|e| e.to_string())?;
        self.db.execute("UPDATE plotlines SET scenes = ? WHERE id = ?", &[&json, &plotline_id])?;
        self.replace_plotline_scenes(plotline_id, scenes)
    }

    /// Persist a plotline status change.
    pub fn update_plotline_status(&self, plotline_id: &str, status: &str) -> Result<(), String> {
        self.db.execute("UPDATE plotlines SET status = ? WHERE id = ?", &[&status, &plotline_id])?;
        Ok(())
    }

    fn load_plotline_scenes(&self, plotline_id: &str) -> Result<Vec<AdventureScene>, String> {
        let rows = self.db.fetch_all(
            "SELECT scene_json FROM plotline_scenes WHERE plotline_id = ? ORDER BY scene_index",
            &[&plotline_id],
        )?;
        if !rows.is_empty() {
            return Ok(rows
                .iter()
                .filter_map(|row| serde_json::from_str(row.get("scene_json").and_then(|v| v.as_str())?).ok())
                .collect());
        }
        let legacy = self.db.fetch_one("SELECT scenes FROM plotlines WHERE id = ?", &[&plotline_id])?;
        let Some(legacy) = legacy else { return Ok(vec![]) };
        Ok(serde_json::from_str(legacy.get("scenes").and_then(|v| v.as_str()).unwrap_or("[]")).unwrap_or_default())
    }

    fn replace_plotline_scenes(
        &self,
        plotline_id: &str,
        scenes: &[AdventureScene],
    ) -> Result<(), String> {
        self.db.execute("DELETE FROM plotline_scenes WHERE plotline_id = ?", &[&plotline_id])?;
        for (index, scene) in scenes.iter().enumerate() {
            let json = serde_json::to_string(scene).map_err(|e| e.to_string())?;
            let idx = index as i64;
            self.db.execute(
                "INSERT INTO plotline_scenes (plotline_id, scene_index, scene_json) VALUES (?, ?, ?)",
                &[&plotline_id, &idx, &json],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threads_crud_and_prefix_find() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = OracleRepository::new(&db);
        repo.create_thread("thread-abc", "story", "The Heist").unwrap();
        assert_eq!(repo.list_threads(None).unwrap().len(), 1);
        assert_eq!(repo.list_threads(Some("active")).unwrap().len(), 1);
        repo.update_thread_progress("thread-abc", 3).unwrap();
        repo.update_thread_status("thread-abc", "resolved").unwrap();
        let found = repo.find_thread("thread-a").unwrap().unwrap(); // unique prefix
        assert_eq!(found.progress, 3);
        assert_eq!(found.status, "resolved");
    }

    #[test]
    fn plotline_scenes_round_trip() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = OracleRepository::new(&db);
        repo.create_plotline("pl1", "Rising Dark", "betrayal").unwrap();
        let scenes = vec![AdventureScene {
            theme: "betrayal".into(),
            element: "ally turns".into(),
            character: "the captain".into(),
            plot: "a knife in the dark".into(),
            narration: "...".into(),
        }];
        repo.update_plotline_scenes("pl1", &scenes).unwrap();
        let pl = repo.find_plotline("pl1").unwrap().unwrap();
        assert_eq!(pl.scenes.len(), 1);
        assert_eq!(pl.scenes[0].theme, "betrayal");
    }

    #[test]
    fn oracle_npcs_crud() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = OracleRepository::new(&db);
        repo.create_oracle_npc("npc-1", "Mara", "a fence", Some("e1")).unwrap();
        assert_eq!(repo.list_oracle_npcs(None).unwrap().len(), 1);
        assert_eq!(repo.find_oracle_npc("npc-1").unwrap().unwrap().name, "Mara");
        repo.delete_oracle_npc("npc-1").unwrap();
        assert!(repo.find_oracle_npc("npc-1").unwrap().is_none());
    }
}
