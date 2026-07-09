//! CRUD repository for factions, assets, relations, and reputation.
//!
//! Ported from `src/harsh_realm/faction/repository.py` and its asset/crud/
//! relation mixins (combined — Rust has no mixin inheritance).

use std::collections::BTreeMap;

use rusqlite::ToSql;

use crate::admin::config::FactionAssetStat;
use crate::admin::faction::FactionRelationRecord;
use crate::db::{Row, WorldDatabase};
use crate::editor_api::factions::{FactionAssetUpdateRequest, FactionUpdateRequest};
use crate::faction::{FactionAssetData, FactionAssetState, FactionData, FactionRelationState};
use crate::runtime::{JsonObject, JsonValue};

/// CRUD operations for factions, assets, relations, and reputation.
pub struct FactionRepository<'a> {
    db: &'a WorldDatabase,
}

fn rs(row: &Row, key: &str) -> String {
    row.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}
fn ri(row: &Row, key: &str) -> i64 {
    row.get(key).and_then(|v| v.as_i64()).unwrap_or(0)
}
fn roi(row: &Row, key: &str) -> Option<i32> {
    row.get(key).and_then(|v| v.as_i64()).map(|x| x as i32)
}

fn sorted_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

impl<'a> FactionRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        FactionRepository { db }
    }

    // --- factions ---

    /// Insert a faction (with goal/tag child rows).
    pub fn create_faction(&self, faction: &FactionData) -> Result<(), String> {
        self.db.execute(
            "INSERT INTO factions (id, name, hp, max_hp, force, cunning, wealth, xp, home_q, home_r) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            &[
                &faction.id, &faction.name, &(faction.hp as i64), &(faction.max_hp as i64),
                &(faction.force as i64), &(faction.cunning as i64), &(faction.wealth as i64),
                &(faction.xp as i64), &faction.home_q.map(|x| x as i64), &faction.home_r.map(|x| x as i64),
            ],
        )?;
        self.replace_goals(&faction.id, &faction.goals)?;
        self.replace_tags(&faction.id, &faction.tags)?;
        Ok(())
    }

    /// Return a faction by id (with goals/tags), or `None`.
    pub fn get_faction(&self, faction_id: &str) -> Result<Option<FactionData>, String> {
        let row = self.db.fetch_one("SELECT * FROM factions WHERE id = ?", &[&faction_id])?;
        let Some(row) = row else { return Ok(None) };
        Ok(Some(self.faction_from_row(&row, self.load_goals(faction_id)?, self.load_tags(faction_id)?)))
    }

    /// Return all factions ordered by name.
    pub fn list_factions(&self) -> Result<Vec<FactionData>, String> {
        let rows = self.db.fetch_all("SELECT * FROM factions ORDER BY name", &[])?;
        let goal_map = self.load_child_map("faction_goals", "goal", "goal_index")?;
        let tag_map = self.load_child_map("faction_tags", "tag", "tag_index")?;
        Ok(rows
            .iter()
            .map(|row| {
                let id = rs(row, "id");
                self.faction_from_row(
                    row,
                    goal_map.get(&id).cloned().unwrap_or_default(),
                    tag_map.get(&id).cloned().unwrap_or_default(),
                )
            })
            .collect())
    }

    /// Update fields on a faction and return the updated data.
    pub fn update_faction(&self, faction_id: &str, req: &FactionUpdateRequest) -> Result<Option<FactionData>, String> {
        let mut parts: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        macro_rules! push_opt_str { ($col:literal, $v:expr) => { if let Some(v) = &$v { parts.push(concat!($col, " = ?").into()); params.push(Box::new(v.clone())); } } }
        macro_rules! push_opt_int { ($col:literal, $v:expr) => { if let Some(v) = $v { parts.push(concat!($col, " = ?").into()); params.push(Box::new(v as i64)); } } }
        push_opt_str!("name", req.name);
        push_opt_int!("hp", req.hp);
        push_opt_int!("max_hp", req.max_hp);
        push_opt_int!("force", req.force);
        push_opt_int!("cunning", req.cunning);
        push_opt_int!("wealth", req.wealth);
        push_opt_int!("xp", req.xp);
        push_opt_int!("home_q", req.home_q);
        push_opt_int!("home_r", req.home_r);
        if !parts.is_empty() {
            params.push(Box::new(faction_id.to_string()));
            let sql = format!("UPDATE factions SET {} WHERE id = ?", parts.join(", "));
            let refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
            self.db.execute(&sql, &refs)?;
        }
        if let Some(goals) = &req.goals {
            self.replace_goals(faction_id, goals)?;
        }
        if let Some(tags) = &req.tags {
            self.replace_tags(faction_id, tags)?;
        }
        self.get_faction(faction_id)
    }

    /// Delete a faction by id and all rows that reference it: goals, tags,
    /// assets (and their asset-state), relations, and relation history. (FK
    /// enforcement is off, so these cascades are done explicitly.)
    pub fn delete_faction(&self, faction_id: &str) -> Result<(), String> {
        self.db.execute("DELETE FROM faction_goals WHERE faction_id = ?", &[&faction_id])?;
        self.db.execute("DELETE FROM faction_tags WHERE faction_id = ?", &[&faction_id])?;
        // Asset state is keyed by asset_id → remove it before the assets it points to.
        self.db.execute(
            "DELETE FROM faction_asset_state WHERE asset_id IN \
             (SELECT id FROM faction_assets WHERE faction_id = ?)",
            &[&faction_id],
        )?;
        self.db.execute("DELETE FROM faction_assets WHERE faction_id = ?", &[&faction_id])?;
        self.db.execute(
            "DELETE FROM faction_relations WHERE faction_a = ? OR faction_b = ?",
            &[&faction_id, &faction_id],
        )?;
        self.db.execute(
            "DELETE FROM faction_relation_history WHERE faction_a = ? OR faction_b = ?",
            &[&faction_id, &faction_id],
        )?;
        self.db.execute("DELETE FROM factions WHERE id = ?", &[&faction_id])?;
        Ok(())
    }

    fn faction_from_row(&self, row: &Row, goals: Vec<String>, tags: Vec<String>) -> FactionData {
        FactionData {
            id: rs(row, "id"),
            name: rs(row, "name"),
            hp: ri(row, "hp") as i32,
            max_hp: ri(row, "max_hp") as i32,
            force: ri(row, "force") as i32,
            cunning: ri(row, "cunning") as i32,
            wealth: ri(row, "wealth") as i32,
            xp: ri(row, "xp") as i32,
            home_q: roi(row, "home_q"),
            home_r: roi(row, "home_r"),
            goals,
            tags,
            data: JsonObject::new(),
        }
    }

    fn load_child(&self, table: &str, col: &str, index_col: &str, faction_id: &str, legacy_col: &str) -> Result<Vec<String>, String> {
        let sql = format!("SELECT {col} FROM {table} WHERE faction_id = ? ORDER BY {index_col}");
        let rows = self.db.fetch_all(&sql, &[&faction_id])?;
        if !rows.is_empty() {
            return Ok(rows.iter().map(|r| rs(r, col)).collect());
        }
        let legacy = self.db.fetch_one("SELECT goals, tags FROM factions WHERE id = ?", &[&faction_id])?;
        let Some(legacy) = legacy else { return Ok(vec![]) };
        Ok(serde_json::from_str(legacy.get(legacy_col).and_then(|v| v.as_str()).unwrap_or("[]")).unwrap_or_default())
    }

    fn load_goals(&self, faction_id: &str) -> Result<Vec<String>, String> {
        self.load_child("faction_goals", "goal", "goal_index", faction_id, "goals")
    }
    fn load_tags(&self, faction_id: &str) -> Result<Vec<String>, String> {
        self.load_child("faction_tags", "tag", "tag_index", faction_id, "tags")
    }

    fn load_child_map(&self, table: &str, col: &str, index_col: &str) -> Result<BTreeMap<String, Vec<String>>, String> {
        let sql = format!("SELECT faction_id, {col} FROM {table} ORDER BY faction_id, {index_col}");
        let rows = self.db.fetch_all(&sql, &[])?;
        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for row in &rows {
            map.entry(rs(row, "faction_id")).or_default().push(rs(row, col));
        }
        Ok(map)
    }

    fn replace_goals(&self, faction_id: &str, goals: &[String]) -> Result<(), String> {
        self.db.execute("DELETE FROM faction_goals WHERE faction_id = ?", &[&faction_id])?;
        for (i, goal) in goals.iter().enumerate() {
            let idx = i as i64;
            self.db.execute("INSERT INTO faction_goals (faction_id, goal_index, goal) VALUES (?, ?, ?)", &[&faction_id, &idx, goal])?;
        }
        Ok(())
    }
    fn replace_tags(&self, faction_id: &str, tags: &[String]) -> Result<(), String> {
        self.db.execute("DELETE FROM faction_tags WHERE faction_id = ?", &[&faction_id])?;
        for (i, tag) in tags.iter().enumerate() {
            let idx = i as i64;
            self.db.execute("INSERT INTO faction_tags (faction_id, tag_index, tag) VALUES (?, ?, ?)", &[&faction_id, &idx, tag])?;
        }
        Ok(())
    }

    // --- assets ---

    /// Insert a faction asset (with its typed state).
    pub fn create_asset(&self, asset: &FactionAssetData, state: FactionAssetState) -> Result<(), String> {
        self.db.execute(
            "INSERT INTO faction_assets (id, faction_id, asset_type, category, hp, max_hp, location_q, location_r) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            &[
                &asset.id, &asset.faction_id, &asset.asset_type, &asset.category,
                &(asset.hp as i64), &(asset.max_hp as i64),
                &asset.location_q.map(|x| x as i64), &asset.location_r.map(|x| x as i64),
            ],
        )?;
        self.save_asset_state(&asset.id, state)
    }

    /// Return an asset by id, or `None`.
    pub fn get_asset(&self, asset_id: &str) -> Result<Option<FactionAssetData>, String> {
        let row = self.db.fetch_one("SELECT * FROM faction_assets WHERE id = ?", &[&asset_id])?;
        let Some(row) = row else { return Ok(None) };
        Ok(Some(self.asset_from_row(&row, self.load_asset_state(asset_id)?)))
    }

    /// Return assets, optionally filtered by faction id.
    pub fn list_assets(&self, faction_id: Option<&str>) -> Result<Vec<FactionAssetData>, String> {
        let rows = match faction_id {
            Some(fid) => self.db.fetch_all("SELECT * FROM faction_assets WHERE faction_id = ? ORDER BY asset_type", &[&fid])?,
            None => self.db.fetch_all("SELECT * FROM faction_assets ORDER BY asset_type", &[])?,
        };
        let state_map = self.load_asset_state_map()?;
        Ok(rows
            .iter()
            .map(|row| {
                let id = rs(row, "id");
                self.asset_from_row(row, state_map.get(&id).copied().unwrap_or_default())
            })
            .collect())
    }

    /// Update fields on an asset and return the updated data.
    pub fn update_asset(
        &self,
        asset_id: &str,
        req: &FactionAssetUpdateRequest,
        state: Option<FactionAssetState>,
    ) -> Result<Option<FactionAssetData>, String> {
        let mut parts: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        if let Some(v) = &req.asset_type { parts.push("asset_type = ?".into()); params.push(Box::new(v.clone())); }
        if let Some(v) = &req.category { parts.push("category = ?".into()); params.push(Box::new(v.clone())); }
        for (col, v) in [("hp", req.hp), ("max_hp", req.max_hp), ("location_q", req.location_q), ("location_r", req.location_r)] {
            if let Some(val) = v { parts.push(format!("{col} = ?")); params.push(Box::new(val as i64)); }
        }
        if !parts.is_empty() {
            params.push(Box::new(asset_id.to_string()));
            let sql = format!("UPDATE faction_assets SET {} WHERE id = ?", parts.join(", "));
            let refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
            self.db.execute(&sql, &refs)?;
        }
        if let Some(state) = state {
            self.save_asset_state(asset_id, state)?;
        }
        self.get_asset(asset_id)
    }

    /// Delete an asset by id.
    pub fn delete_asset(&self, asset_id: &str) -> Result<(), String> {
        self.db.execute("DELETE FROM faction_asset_state WHERE asset_id = ?", &[&asset_id])?;
        self.db.execute("DELETE FROM faction_assets WHERE id = ?", &[&asset_id])?;
        Ok(())
    }

    fn asset_from_row(&self, row: &Row, state: FactionAssetState) -> FactionAssetData {
        let mut data = JsonObject::new();
        data.insert("expanded".into(), JsonValue::from(state.expanded));
        FactionAssetData {
            id: rs(row, "id"),
            faction_id: rs(row, "faction_id"),
            asset_type: rs(row, "asset_type"),
            category: rs(row, "category"),
            hp: ri(row, "hp") as i32,
            max_hp: ri(row, "max_hp") as i32,
            location_q: roi(row, "location_q"),
            location_r: roi(row, "location_r"),
            data,
        }
    }

    fn load_asset_state(&self, asset_id: &str) -> Result<FactionAssetState, String> {
        let row = self.db.fetch_one("SELECT expanded FROM faction_asset_state WHERE asset_id = ?", &[&asset_id])?;
        Ok(FactionAssetState { expanded: row.map(|r| ri(&r, "expanded") != 0).unwrap_or(false) })
    }

    fn load_asset_state_map(&self) -> Result<BTreeMap<String, FactionAssetState>, String> {
        let rows = self.db.fetch_all("SELECT asset_id, expanded FROM faction_asset_state", &[])?;
        Ok(rows.iter().map(|r| (rs(r, "asset_id"), FactionAssetState { expanded: ri(r, "expanded") != 0 })).collect())
    }

    fn save_asset_state(&self, asset_id: &str, state: FactionAssetState) -> Result<(), String> {
        let expanded = state.expanded as i64;
        self.db.execute(
            "INSERT INTO faction_asset_state (asset_id, expanded) VALUES (?, ?) \
             ON CONFLICT(asset_id) DO UPDATE SET expanded = excluded.expanded",
            &[&asset_id, &expanded],
        )?;
        Ok(())
    }

    /// Return one asset-stat row by asset type.
    pub fn get_asset_stats(&self, asset_type: &str) -> Result<Option<FactionAssetStat>, String> {
        let row = self.db.fetch_one("SELECT * FROM faction_asset_stats WHERE asset_type = ?", &[&asset_type])?;
        Ok(row.as_ref().map(asset_stat_from_row))
    }

    /// Return all asset-stat rows ordered by cost descending.
    pub fn list_asset_stats(&self) -> Result<Vec<FactionAssetStat>, String> {
        let rows = self.db.fetch_all("SELECT * FROM faction_asset_stats ORDER BY cost DESC", &[])?;
        Ok(rows.iter().map(asset_stat_from_row).collect())
    }

    // --- relations & reputation ---

    /// Return the disposition between two factions (default `"neutral"`).
    pub fn get_relation(&self, faction_a: &str, faction_b: &str) -> Result<String, String> {
        let (a, b) = sorted_pair(faction_a, faction_b);
        let row = self.db.fetch_one(
            "SELECT disposition FROM faction_relations WHERE faction_a = ? AND faction_b = ?",
            &[&a, &b],
        )?;
        Ok(row.map(|r| rs(&r, "disposition")).unwrap_or_else(|| "neutral".to_string()))
    }

    /// Return the full relation record between two factions.
    pub fn get_relation_record(&self, faction_a: &str, faction_b: &str) -> Result<FactionRelationRecord, String> {
        let (a, b) = sorted_pair(faction_a, faction_b);
        let row = self.db.fetch_one(
            "SELECT disposition FROM faction_relations WHERE faction_a = ? AND faction_b = ?",
            &[&a, &b],
        )?;
        let state = self.load_relation_state(&a, &b)?;
        Ok(FactionRelationRecord {
            faction_a: a,
            faction_b: b,
            disposition: row.map(|r| rs(&r, "disposition")).unwrap_or_else(|| "neutral".to_string()),
            history: state.history,
        })
    }

    /// Set the disposition between two factions (upsert).
    pub fn set_relation(
        &self,
        faction_a: &str,
        faction_b: &str,
        disposition: &str,
        history: Option<&[String]>,
    ) -> Result<FactionRelationRecord, String> {
        let (a, b) = sorted_pair(faction_a, faction_b);
        self.db.execute(
            "INSERT INTO faction_relations (faction_a, faction_b, disposition) VALUES (?, ?, ?) \
             ON CONFLICT(faction_a, faction_b) DO UPDATE SET disposition = excluded.disposition",
            &[&a, &b, &disposition],
        )?;
        if let Some(history) = history {
            self.replace_relation_history(&a, &b, history)?;
        }
        self.get_relation_record(&a, &b)
    }

    /// Return the player's reputation with a faction (default 0).
    pub fn get_reputation(&self, entity_id: &str, faction_id: &str) -> Result<i64, String> {
        let row = self.db.fetch_one(
            "SELECT score FROM reputation WHERE entity_id = ? AND faction_id = ?",
            &[&entity_id, &faction_id],
        )?;
        Ok(row.map(|r| ri(&r, "score")).unwrap_or(0))
    }

    /// Set the player's reputation with a faction (upsert).
    pub fn set_reputation(&self, entity_id: &str, faction_id: &str, score: i64) -> Result<(), String> {
        self.db.execute(
            "INSERT INTO reputation (entity_id, faction_id, score) VALUES (?, ?, ?) \
             ON CONFLICT(entity_id, faction_id) DO UPDATE SET score = excluded.score",
            &[&entity_id, &faction_id, &score],
        )?;
        Ok(())
    }

    /// Adjust reputation by `delta` and return the new score.
    pub fn adjust_reputation(&self, entity_id: &str, faction_id: &str, delta: i64) -> Result<i64, String> {
        let new_score = self.get_reputation(entity_id, faction_id)? + delta;
        self.set_reputation(entity_id, faction_id, new_score)?;
        Ok(new_score)
    }

    /// Return encounter tag modifiers for a faction disposition.
    pub fn get_encounter_weights_for_disposition(&self, disposition: &str) -> Result<BTreeMap<String, i64>, String> {
        let rows = self.db.fetch_all(
            "SELECT encounter_tag, weight_modifier FROM encounter_weights WHERE faction_disposition = ?",
            &[&disposition],
        )?;
        Ok(rows.iter().map(|r| (rs(r, "encounter_tag"), ri(r, "weight_modifier"))).collect())
    }

    fn load_relation_state(&self, faction_a: &str, faction_b: &str) -> Result<FactionRelationState, String> {
        let rows = self.db.fetch_all(
            "SELECT note FROM faction_relation_history WHERE faction_a = ? AND faction_b = ? ORDER BY history_index",
            &[&faction_a, &faction_b],
        )?;
        if !rows.is_empty() {
            return Ok(FactionRelationState { history: rows.iter().map(|r| rs(r, "note")).collect() });
        }
        let legacy = self.db.fetch_one(
            "SELECT history FROM faction_relations WHERE faction_a = ? AND faction_b = ?",
            &[&faction_a, &faction_b],
        )?;
        let Some(legacy) = legacy else { return Ok(FactionRelationState::default()) };
        Ok(FactionRelationState {
            history: serde_json::from_str(legacy.get("history").and_then(|v| v.as_str()).unwrap_or("[]")).unwrap_or_default(),
        })
    }

    fn replace_relation_history(&self, faction_a: &str, faction_b: &str, history: &[String]) -> Result<(), String> {
        self.db.execute(
            "DELETE FROM faction_relation_history WHERE faction_a = ? AND faction_b = ?",
            &[&faction_a, &faction_b],
        )?;
        for (i, note) in history.iter().enumerate() {
            let idx = i as i64;
            self.db.execute(
                "INSERT INTO faction_relation_history (faction_a, faction_b, history_index, note) VALUES (?, ?, ?, ?)",
                &[&faction_a, &faction_b, &idx, note],
            )?;
        }
        Ok(())
    }
}

fn asset_stat_from_row(row: &Row) -> FactionAssetStat {
    FactionAssetStat {
        asset_type: rs(row, "asset_type"),
        category: rs(row, "category"),
        min_attribute: ri(row, "min_attribute") as i32,
        cost: ri(row, "cost") as i32,
        upkeep: ri(row, "upkeep") as i32,
        max_hp: ri(row, "max_hp") as i32,
        attack_stat: rs(row, "attack_stat"),
        counter_stat: rs(row, "counter_stat"),
        attack_roll: rs(row, "attack_roll"),
        special: rs(row, "special"),
        description: rs(row, "description"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn faction(id: &str, name: &str) -> FactionData {
        FactionData {
            id: id.into(),
            name: name.into(),
            hp: 7,
            max_hp: 7,
            force: 1,
            cunning: 2,
            wealth: 3,
            xp: 0,
            home_q: Some(1),
            home_r: Some(2),
            goals: vec!["expand".into()],
            tags: vec!["raiders".into()],
            data: JsonObject::new(),
        }
    }

    #[test]
    fn faction_crud_with_goals_tags() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = FactionRepository::new(&db);
        repo.create_faction(&faction("f1", "Reavers")).unwrap();
        let got = repo.get_faction("f1").unwrap().unwrap();
        assert_eq!(got.name, "Reavers");
        assert_eq!(got.goals, vec!["expand".to_string()]);
        assert_eq!(got.tags, vec!["raiders".to_string()]);

        let mut upd = FactionUpdateRequest::default();
        upd.name = Some("Red Reavers".into());
        upd.tags = Some(vec!["raiders".into(), "coastal".into()]);
        let updated = repo.update_faction("f1", &upd).unwrap().unwrap();
        assert_eq!(updated.name, "Red Reavers");
        assert_eq!(updated.tags.len(), 2);

        assert_eq!(repo.list_factions().unwrap().len(), 1);
        repo.delete_faction("f1").unwrap();
        assert!(repo.get_faction("f1").unwrap().is_none());
    }

    #[test]
    fn assets_and_relations_and_reputation() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = FactionRepository::new(&db);
        let asset = FactionAssetData {
            id: "a1".into(),
            faction_id: "f1".into(),
            asset_type: "warband".into(),
            category: "force".into(),
            hp: 4,
            max_hp: 4,
            location_q: None,
            location_r: None,
            data: JsonObject::new(),
        };
        repo.create_asset(&asset, FactionAssetState { expanded: true }).unwrap();
        let got = repo.get_asset("a1").unwrap().unwrap();
        assert_eq!(got.asset_type, "warband");
        assert_eq!(got.data.get("expanded"), Some(&JsonValue::from(true)));
        assert_eq!(repo.list_assets(Some("f1")).unwrap().len(), 1);

        // relations are stored with the id pair sorted
        repo.set_relation("zeta", "alpha", "hostile", Some(&["raided".into()])).unwrap();
        assert_eq!(repo.get_relation("alpha", "zeta").unwrap(), "hostile");
        let rec = repo.get_relation_record("alpha", "zeta").unwrap();
        assert_eq!(rec.history, vec!["raided".to_string()]);

        assert_eq!(repo.adjust_reputation("hero", "f1", 3).unwrap(), 3);
        assert_eq!(repo.adjust_reputation("hero", "f1", -1).unwrap(), 2);
    }

    #[test]
    fn delete_faction_cascades_assets_state_and_relations() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = FactionRepository::new(&db);
        repo.create_faction(&faction("f1", "Reavers")).unwrap();
        repo.create_faction(&faction("f2", "Wardens")).unwrap();
        let asset = FactionAssetData {
            id: "a1".into(),
            faction_id: "f1".into(),
            asset_type: "warband".into(),
            category: "force".into(),
            hp: 4,
            max_hp: 4,
            location_q: None,
            location_r: None,
            data: JsonObject::new(),
        };
        repo.create_asset(&asset, FactionAssetState { expanded: true }).unwrap();
        repo.set_relation("f1", "f2", "hostile", Some(&["raided".into()])).unwrap();
        assert_eq!(repo.list_assets(Some("f1")).unwrap().len(), 1);

        repo.delete_faction("f1").unwrap();

        // The faction's asset, its asset-state, and any relation referencing it
        // are all removed (no orphans).
        assert!(repo.get_asset("a1").unwrap().is_none(), "asset should be deleted");
        assert!(
            db.fetch_one("SELECT asset_id FROM faction_asset_state WHERE asset_id = ?", &[&"a1"])
                .unwrap()
                .is_none(),
            "asset_state should be deleted"
        );
        assert!(
            db.fetch_one(
                "SELECT 1 AS x FROM faction_relations WHERE faction_a = ? OR faction_b = ?",
                &[&"f1", &"f1"],
            )
            .unwrap()
            .is_none(),
            "relation referencing the faction should be deleted"
        );
    }
}
