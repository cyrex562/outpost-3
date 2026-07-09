//! Quest lifecycle service — owns quest-instance persistence and transitions.
//!
//! The service performs pure DB + content logic and returns the affected
//! [`ActiveQuest`]; the domain-event handlers turn results into client-facing
//! `quest.*` events. Mirrors the `status_effects` service split.

use crate::runtime::JsonObject;
use serde_json::Value as JsonValue;

use super::catalog::QuestContent;
use super::models::{
    ActiveQuest, STATUS_ACCEPTED, STATUS_COMPLETED, STATUS_FAILED, STATUS_IN_PROGRESS,
};
use super::repository::QuestRepository;

/// Minimal world-clock interface used by the quest service.
pub trait WorldClock {
    /// Current world tick.
    fn tick(&self) -> i32;
}

/// Owns quest-instance persistence and lifecycle transitions.
pub struct QuestService<'a, Q: QuestContent, K: WorldClock> {
    repo: QuestRepository<'a>,
    content: Q,
    clock: Option<K>,
}

impl<'a, Q: QuestContent, K: WorldClock> QuestService<'a, Q, K> {
    /// Durable subsystem marker.
    pub const PERSISTENCE: &'static str = "durable";

    /// Construct over a repository, quest content, and optional clock.
    pub fn new(repo: QuestRepository<'a>, content: Q, clock: Option<K>) -> Self {
        QuestService {
            repo,
            content,
            clock,
        }
    }

    fn current_tick(&self) -> i32 {
        self.clock.as_ref().map(|c| c.tick()).unwrap_or(0)
    }

    /// Accept a quest for an entity.
    ///
    /// Fails when the quest id is unknown, the quest is already active for the
    /// entity, a prerequisite quest is not yet completed, or the quest's
    /// `expiry_tick` has passed.
    pub fn accept(&self, entity_id: &str, quest_id: &str) -> Result<ActiveQuest, String> {
        let quest = self
            .content
            .get(quest_id)
            .ok_or_else(|| format!("unknown quest '{quest_id}'"))?;

        if self.repo.find_active(entity_id, quest_id)?.is_some() {
            return Err(format!("quest '{quest_id}' is already active"));
        }

        let tick = self.current_tick();
        if let Some(expiry) = quest.expiry_tick {
            if tick > expiry {
                return Err(format!("quest '{quest_id}' has expired"));
            }
        }

        if !quest.prerequisites.is_empty() {
            let completed: std::collections::BTreeSet<String> = self
                .repo
                .list_completed(entity_id)?
                .into_iter()
                .map(|q| q.quest_id)
                .collect();
            for prereq in &quest.prerequisites {
                if !completed.contains(prereq) {
                    return Err(format!(
                        "quest '{quest_id}' requires '{prereq}' to be completed first"
                    ));
                }
            }
        }

        self.repo
            .insert(entity_id, quest_id, STATUS_ACCEPTED, tick, &JsonObject::new())
    }

    /// Record progress on an objective, advancing the quest to `in_progress`.
    pub fn update_progress(
        &self,
        entity_id: &str,
        quest_id: &str,
        objective_key: &str,
        value: i32,
    ) -> Result<ActiveQuest, String> {
        let mut quest = self
            .repo
            .find_active(entity_id, quest_id)?
            .ok_or_else(|| format!("no active quest '{quest_id}' for entity"))?;

        quest
            .progress
            .insert(objective_key.to_string(), JsonValue::from(value));
        self.repo.set_progress(quest.id, &quest.progress)?;

        if quest.status == STATUS_ACCEPTED {
            self.repo.set_status(quest.id, STATUS_IN_PROGRESS, None)?;
            quest.status = STATUS_IN_PROGRESS.to_string();
        }
        Ok(quest)
    }

    /// Mark an active quest completed.
    pub fn complete(&self, entity_id: &str, quest_id: &str) -> Result<ActiveQuest, String> {
        self.terminate(entity_id, quest_id, STATUS_COMPLETED)
    }

    /// Mark an active quest failed (also used for `abandon`).
    pub fn fail(&self, entity_id: &str, quest_id: &str) -> Result<ActiveQuest, String> {
        self.terminate(entity_id, quest_id, STATUS_FAILED)
    }

    fn terminate(
        &self,
        entity_id: &str,
        quest_id: &str,
        status: &str,
    ) -> Result<ActiveQuest, String> {
        let mut quest = self
            .repo
            .find_active(entity_id, quest_id)?
            .ok_or_else(|| format!("no active quest '{quest_id}' for entity"))?;
        let tick = self.current_tick();
        self.repo.set_status(quest.id, status, Some(tick))?;
        quest.status = status.to_string();
        quest.completed_tick = Some(tick);
        Ok(quest)
    }

    /// The entity's active (accepted / in-progress) quests.
    pub fn list_active(&self, entity_id: &str) -> Result<Vec<ActiveQuest>, String> {
        self.repo.list_active(entity_id)
    }

    /// The quest content for `quest_id`, if any (for building event payloads).
    pub fn quest_content(&self, quest_id: &str) -> Option<super::schema::Quest> {
        self.content.get(quest_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::db_schema::SCHEMA_SQL;
    use crate::quest::catalog::QuestCatalog;
    use crate::quest::models::{is_active_status, QUEST_STATUSES};
    use crate::quest::schema::Quest;

    struct Clock(i32);
    impl WorldClock for Clock {
        fn tick(&self) -> i32 {
            self.0
        }
    }

    fn db() -> WorldDatabase {
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        db
    }

    fn quest(id: &str, prereqs: &[&str]) -> Quest {
        Quest {
            id: id.to_string(),
            name: format!("Quest {id}"),
            description: String::new(),
            objectives: vec![],
            reward_gold: 10,
            reward_xp: 20,
            reward_items: vec![],
            prerequisites: prereqs.iter().map(|s| s.to_string()).collect(),
            expiry_tick: None,
            faction_id: None,
        }
    }

    fn service(db: &WorldDatabase, catalog: QuestCatalog) -> QuestService<'_, QuestCatalog, Clock> {
        QuestService::new(QuestRepository::new(db), catalog, Some(Clock(5)))
    }

    #[test]
    fn accept_puts_quest_in_active_list() {
        let db = db();
        let svc = service(&db, QuestCatalog::from_quests(vec![quest("a", &[])]));
        let q = svc.accept("hero", "a").unwrap();
        assert_eq!(q.status, STATUS_ACCEPTED);
        assert_eq!(q.accepted_tick, 5);
        let active = svc.list_active("hero").unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].quest_id, "a");
    }

    #[test]
    fn accept_rejects_unknown_and_duplicate() {
        let db = db();
        let svc = service(&db, QuestCatalog::from_quests(vec![quest("a", &[])]));
        assert!(svc.accept("hero", "nope").is_err(), "unknown quest rejected");
        svc.accept("hero", "a").unwrap();
        assert!(svc.accept("hero", "a").is_err(), "duplicate accept rejected");
    }

    #[test]
    fn accept_enforces_prerequisites() {
        let db = db();
        let svc = service(
            &db,
            QuestCatalog::from_quests(vec![quest("a", &[]), quest("b", &["a"])]),
        );
        // b requires a completed a.
        assert!(svc.accept("hero", "b").is_err(), "prereq not met → rejected");
        svc.accept("hero", "a").unwrap();
        svc.complete("hero", "a").unwrap();
        assert!(svc.accept("hero", "b").is_ok(), "prereq met → accepted");
    }

    #[test]
    fn update_progress_moves_to_in_progress_and_records_value() {
        let db = db();
        let svc = service(&db, QuestCatalog::from_quests(vec![quest("a", &[])]));
        svc.accept("hero", "a").unwrap();
        let q = svc.update_progress("hero", "a", "kills", 3).unwrap();
        assert_eq!(q.status, STATUS_IN_PROGRESS);
        assert_eq!(q.progress.get("kills").and_then(|v| v.as_i64()), Some(3));
        // Still active (in_progress is non-terminal).
        assert_eq!(svc.list_active("hero").unwrap().len(), 1);
    }

    #[test]
    fn completed_quest_leaves_active_list() {
        let db = db();
        let svc = service(&db, QuestCatalog::from_quests(vec![quest("a", &[])]));
        svc.accept("hero", "a").unwrap();
        let q = svc.complete("hero", "a").unwrap();
        assert_eq!(q.status, STATUS_COMPLETED);
        assert_eq!(q.completed_tick, Some(5));
        assert!(
            svc.list_active("hero").unwrap().is_empty(),
            "completed quests never appear in list_active"
        );
        assert!(svc.complete("hero", "a").is_err(), "cannot complete twice");
    }

    #[test]
    fn failed_quest_leaves_active_list() {
        let db = db();
        let svc = service(&db, QuestCatalog::from_quests(vec![quest("a", &[])]));
        svc.accept("hero", "a").unwrap();
        svc.fail("hero", "a").unwrap();
        assert!(svc.list_active("hero").unwrap().is_empty());
    }

    // ---- invariants --------------------------------------------------------

    #[test]
    fn status_is_always_a_member_of_the_state_set() {
        let db = db();
        let svc = service(
            &db,
            QuestCatalog::from_quests(vec![quest("a", &[]), quest("b", &[]), quest("c", &[])]),
        );
        svc.accept("hero", "a").unwrap();
        svc.accept("hero", "b").unwrap();
        svc.update_progress("hero", "b", "x", 1).unwrap();
        svc.accept("hero", "c").unwrap();
        svc.complete("hero", "c").unwrap();
        for q in QuestRepository::new(&db).list_all("hero").unwrap() {
            assert!(
                QUEST_STATUSES.contains(&q.status.as_str()),
                "status '{}' must be a member of the defined state set",
                q.status
            );
        }
    }

    #[test]
    fn active_and_completed_never_overlap() {
        let db = db();
        let svc = service(
            &db,
            QuestCatalog::from_quests(vec![quest("a", &[]), quest("b", &[])]),
        );
        svc.accept("hero", "a").unwrap();
        svc.accept("hero", "b").unwrap();
        svc.complete("hero", "b").unwrap();
        let repo = QuestRepository::new(&db);
        let active: std::collections::BTreeSet<_> = repo
            .list_active("hero")
            .unwrap()
            .into_iter()
            .map(|q| q.id)
            .collect();
        let completed: std::collections::BTreeSet<_> = repo
            .list_completed("hero")
            .unwrap()
            .into_iter()
            .map(|q| q.id)
            .collect();
        assert!(active.is_disjoint(&completed), "no quest is both active and completed");
        // Every listed quest's status matches the partition it came from.
        for q in repo.list_active("hero").unwrap() {
            assert!(is_active_status(&q.status));
        }
    }
}
