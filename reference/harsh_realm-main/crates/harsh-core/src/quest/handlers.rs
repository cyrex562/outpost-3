//! Domain-event handlers for quest lifecycle requests (HR-19).
//!
//! Each handler resolves a `quest.*_requested` command intent against a
//! [`QuestService`] and returns the client-facing result event. The GM
//! controller invokes these on-demand from `resolve_domain_events` (mirroring
//! the HR-776 status-effect routing), since the lifetime-bound service cannot be
//! wired at controller construction (content is unavailable until world-load).

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::events::GameEvent;
use crate::payloads::quests::{
    QuestAcceptRequested, QuestAcceptedNotice, QuestCompletedNotice, QuestFailedNotice,
    QuestOutcomeRequested, QuestProgressRequested, QuestProgressUpdatedNotice,
};
use crate::runtime::JsonObject;

use super::catalog::QuestContent;
use super::service::{QuestService, WorldClock};

const SOURCE: &str = "quest";

fn event_data<T: Serialize>(payload: &T) -> JsonObject {
    match serde_json::to_value(payload) {
        Ok(serde_json::Value::Object(map)) => map,
        _ => JsonObject::new(),
    }
}

fn parse_payload<T: DeserializeOwned>(event: &GameEvent) -> Option<T> {
    match serde_json::from_value::<T>(serde_json::Value::Object(event.data.clone())) {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!(
                "quest_handler: '{}' failed to deserialize {}: {e}",
                event.event_type,
                std::any::type_name::<T>()
            );
            None
        }
    }
}

/// `quest.accept_requested` → `quest.accepted`.
pub fn handle_accept_requested<Q: QuestContent, K: WorldClock>(
    service: &QuestService<'_, Q, K>,
    event: &GameEvent,
) -> Vec<GameEvent> {
    let Some(req) = parse_payload::<QuestAcceptRequested>(event) else {
        return vec![];
    };
    let active = match service.accept(&req.entity_id, &req.quest_id) {
        Ok(a) => a,
        Err(_) => return vec![],
    };
    let quest = service.quest_content(&active.quest_id);
    let notice = QuestAcceptedNotice {
        quest_id: active.quest_id.clone(),
        quest_name: quest
            .as_ref()
            .map(|q| q.name.clone())
            .unwrap_or_else(|| active.quest_id.clone()),
        description: quest.map(|q| q.description).unwrap_or_default(),
    };
    vec![GameEvent::new(event.tick, "quest.accepted", event_data(&notice)).with_source(SOURCE)]
}

/// `quest.progress_requested` → `quest.progress_updated`.
pub fn handle_progress_requested<Q: QuestContent, K: WorldClock>(
    service: &QuestService<'_, Q, K>,
    event: &GameEvent,
) -> Vec<GameEvent> {
    let Some(req) = parse_payload::<QuestProgressRequested>(event) else {
        return vec![];
    };
    let updated = match service.update_progress(
        &req.entity_id,
        &req.quest_id,
        &req.objective_key,
        req.value,
    ) {
        Ok(q) => q,
        Err(_) => return vec![],
    };
    let notice = QuestProgressUpdatedNotice {
        quest_id: updated.quest_id.clone(),
        objective_key: req.objective_key,
        value: req.value,
        progress: updated.progress,
    };
    vec![
        GameEvent::new(event.tick, "quest.progress_updated", event_data(&notice))
            .with_source(SOURCE),
    ]
}

/// `quest.complete_requested` → `quest.completed`.
pub fn handle_complete_requested<Q: QuestContent, K: WorldClock>(
    service: &QuestService<'_, Q, K>,
    event: &GameEvent,
) -> Vec<GameEvent> {
    let Some(req) = parse_payload::<QuestOutcomeRequested>(event) else {
        return vec![];
    };
    let active = match service.complete(&req.entity_id, &req.quest_id) {
        Ok(a) => a,
        Err(_) => return vec![],
    };
    let quest = service.quest_content(&active.quest_id);
    let notice = QuestCompletedNotice {
        quest_id: active.quest_id.clone(),
        quest_name: quest
            .as_ref()
            .map(|q| q.name.clone())
            .unwrap_or_else(|| active.quest_id.clone()),
        reward_gold: quest.as_ref().map(|q| q.reward_gold).unwrap_or(0),
        reward_xp: quest.as_ref().map(|q| q.reward_xp).unwrap_or(0),
        reward_items: quest.map(|q| q.reward_items).unwrap_or_default(),
    };
    vec![GameEvent::new(event.tick, "quest.completed", event_data(&notice)).with_source(SOURCE)]
}

/// `quest.fail_requested` → `quest.failed`.
pub fn handle_fail_requested<Q: QuestContent, K: WorldClock>(
    service: &QuestService<'_, Q, K>,
    event: &GameEvent,
) -> Vec<GameEvent> {
    let Some(req) = parse_payload::<QuestOutcomeRequested>(event) else {
        return vec![];
    };
    let active = match service.fail(&req.entity_id, &req.quest_id) {
        Ok(a) => a,
        Err(_) => return vec![],
    };
    let quest = service.quest_content(&active.quest_id);
    let notice = QuestFailedNotice {
        quest_id: active.quest_id.clone(),
        quest_name: quest
            .map(|q| q.name)
            .unwrap_or_else(|| active.quest_id.clone()),
    };
    vec![GameEvent::new(event.tick, "quest.failed", event_data(&notice)).with_source(SOURCE)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::db_schema::SCHEMA_SQL;
    use crate::quest::catalog::QuestCatalog;
    use crate::quest::repository::QuestRepository;
    use crate::quest::schema::Quest;
    use crate::quest::service::{QuestService, WorldClock};

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

    fn quest(id: &str) -> Quest {
        Quest {
            id: id.to_string(),
            name: format!("The {id}"),
            description: "briefing".into(),
            objectives: vec![],
            reward_gold: 15,
            reward_xp: 30,
            reward_items: vec!["relic".into()],
            prerequisites: vec![],
            expiry_tick: None,
            faction_id: None,
        }
    }

    #[test]
    fn accept_requested_emits_quest_accepted_with_content() {
        let db = db();
        let svc = QuestService::new(
            QuestRepository::new(&db),
            QuestCatalog::from_quests(vec![quest("a")]),
            Some(Clock(3)),
        );
        let mut data = JsonObject::new();
        data.insert("entity_id".into(), serde_json::json!("hero"));
        data.insert("quest_id".into(), serde_json::json!("a"));
        let ev = GameEvent::new(3, "quest.accept_requested", data);

        let out = handle_accept_requested(&svc, &ev);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].event_type, "quest.accepted");
        assert_eq!(out[0].data.get("quest_name").and_then(|v| v.as_str()), Some("The a"));
        // The quest is now active in the DB.
        assert_eq!(svc.list_active("hero").unwrap().len(), 1);
    }

    #[test]
    fn complete_requested_emits_quest_completed_with_rewards() {
        let db = db();
        let svc = QuestService::new(
            QuestRepository::new(&db),
            QuestCatalog::from_quests(vec![quest("a")]),
            Some(Clock(3)),
        );
        svc.accept("hero", "a").unwrap();
        let mut data = JsonObject::new();
        data.insert("entity_id".into(), serde_json::json!("hero"));
        data.insert("quest_id".into(), serde_json::json!("a"));
        let ev = GameEvent::new(4, "quest.complete_requested", data);

        let out = handle_complete_requested(&svc, &ev);
        assert_eq!(out[0].event_type, "quest.completed");
        assert_eq!(out[0].data.get("reward_xp").and_then(|v| v.as_i64()), Some(30));
        assert!(svc.list_active("hero").unwrap().is_empty());
    }

    #[test]
    fn accept_requested_emits_nothing_on_failure() {
        let db = db();
        let svc = QuestService::new(
            QuestRepository::new(&db),
            QuestCatalog::from_quests(vec![quest("a")]),
            Some(Clock(0)),
        );
        let mut data = JsonObject::new();
        data.insert("entity_id".into(), serde_json::json!("hero"));
        data.insert("quest_id".into(), serde_json::json!("unknown"));
        let ev = GameEvent::new(0, "quest.accept_requested", data);
        assert!(handle_accept_requested(&svc, &ev).is_empty(), "unknown quest → no event");
    }
}
