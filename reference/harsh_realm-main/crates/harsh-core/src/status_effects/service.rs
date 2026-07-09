//! Durable status effect lifecycle service.
//!
//! Ported from `src/harsh_realm/status_effects/service.py`. The Python service
//! depends on `ContentService` (structural) and an optional `WorldClock`
//! protocol; those are modeled here as the [`ContentLookup`] and [`WorldClock`]
//! traits so the service compiles independently of those subsystems.

use std::collections::BTreeSet;

use crate::runtime::JsonObject;

use super::models::ActiveStatusEffect;
use super::repository::StatusEffectRepository;
use super::schema::{Stacking, StatusEffect};

/// Source of status effect content records keyed by id.
pub trait ContentLookup {
    /// Return the raw content record for `id`, if present.
    fn get(&self, id: &str) -> Option<JsonObject>;
}

/// Minimal world-clock interface used by the status effect service.
pub trait WorldClock {
    /// Return the current world tick.
    fn tick(&self) -> i32;
}

/// Owns active status effect persistence and lifecycle.
pub struct StatusEffectService<'a, C: ContentLookup, K: WorldClock> {
    repo: StatusEffectRepository<'a>,
    content: C,
    clock: Option<K>,
}

impl<'a, C: ContentLookup, K: WorldClock> StatusEffectService<'a, C, K> {
    /// Durability marker matching the Python `PERSISTENCE` attribute.
    pub const PERSISTENCE: &'static str = "durable";

    /// Construct a service over a repository, content lookup, and optional clock.
    pub fn new(repo: StatusEffectRepository<'a>, content: C, clock: Option<K>) -> Self {
        StatusEffectService {
            repo,
            content,
            clock,
        }
    }

    /// Apply an effect to an entity, honoring its stacking policy.
    pub fn apply(
        &self,
        entity_id: &str,
        effect_id: &str,
        duration_ticks: Option<i32>,
        source: Option<&str>,
        data: Option<JsonObject>,
    ) -> Result<ActiveStatusEffect, String> {
        let effect = self.load_effect(effect_id)?;
        let applied_at_tick = self.current_tick();
        let duration = duration_ticks.unwrap_or(effect.default_duration_ticks);
        let expires_at_tick = if duration == 0 {
            None
        } else {
            Some(applied_at_tick + duration)
        };
        let payload = data.unwrap_or_default();

        match effect.stacking {
            Stacking::Replace => {
                self.repo.remove(entity_id, effect_id)?;
                self.repo.add(
                    entity_id,
                    effect_id,
                    applied_at_tick,
                    expires_at_tick,
                    source,
                    payload,
                )
            }
            Stacking::Extend => {
                let existing = self.repo.list_for_entity_effect(entity_id, effect_id)?;
                if let Some(first) = existing.into_iter().next() {
                    let updated = ActiveStatusEffect {
                        expires_at_tick: extended_expiry(
                            first.expires_at_tick,
                            applied_at_tick,
                            duration,
                        ),
                        source: source.map(str::to_string),
                        data: payload,
                        ..first
                    };
                    self.repo.update(&updated)?;
                    return Ok(updated);
                }
                self.repo.add(
                    entity_id,
                    effect_id,
                    applied_at_tick,
                    expires_at_tick,
                    source,
                    payload,
                )
            }
            Stacking::Stack => self.repo.add(
                entity_id,
                effect_id,
                applied_at_tick,
                expires_at_tick,
                source,
                payload,
            ),
        }
    }

    /// Remove all instances of an effect from an entity; returns the count.
    pub fn remove(&self, entity_id: &str, effect_id: &str) -> Result<usize, String> {
        Ok(self.repo.remove(entity_id, effect_id)?.len())
    }

    /// Remove one active effect by row id; returns whether a row was removed.
    pub fn remove_by_id(&self, active_effect_id: i64) -> Result<bool, String> {
        Ok(self.repo.remove_by_id(active_effect_id)?.is_some())
    }

    /// List active effects for an entity.
    pub fn list_for_entity(&self, entity_id: &str) -> Result<Vec<ActiveStatusEffect>, String> {
        self.repo.list_for_entity(entity_id)
    }

    /// Return tags provided by active status effects on an entity.
    pub fn get_tags(&self, entity_id: &str) -> Result<BTreeSet<String>, String> {
        let mut tags: BTreeSet<String> = BTreeSet::new();
        for active in self.list_for_entity(entity_id)? {
            let effect = self.load_effect(&active.effect_id)?;
            tags.extend(effect.provides_tags);
        }
        Ok(tags)
    }

    /// Remove all effects whose expiration tick is due.
    pub fn expire_due(&self, current_tick: i32) -> Result<Vec<ActiveStatusEffect>, String> {
        self.repo.expire_due(current_tick)
    }

    fn load_effect(&self, effect_id: &str) -> Result<StatusEffect, String> {
        let record = self
            .content
            .get(effect_id)
            .ok_or_else(|| format!("Status effect not found: {effect_id}"))?;
        serde_json::from_value(serde_json::Value::Object(record)).map_err(|e| e.to_string())
    }

    fn current_tick(&self) -> i32 {
        self.clock.as_ref().map_or(0, WorldClock::tick)
    }
}

fn extended_expiry(existing_expiry: Option<i32>, current_tick: i32, duration: i32) -> Option<i32> {
    match existing_expiry {
        Some(expiry) if duration != 0 => Some(expiry.max(current_tick) + duration),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use std::collections::HashMap;

    struct MapContent {
        records: HashMap<String, JsonObject>,
    }

    impl ContentLookup for MapContent {
        fn get(&self, id: &str) -> Option<JsonObject> {
            self.records.get(id).cloned()
        }
    }

    struct FixedClock(i32);
    impl WorldClock for FixedClock {
        fn tick(&self) -> i32 {
            self.0
        }
    }

    fn content(stacking: &str, duration: i32) -> MapContent {
        let mut records = HashMap::new();
        let record: JsonObject = serde_json::from_value(serde_json::json!({
            "id": "poison",
            "name": "Poison",
            "default_duration_ticks": duration,
            "provides_tags": ["poisoned"],
            "stacking": stacking,
        }))
        .unwrap();
        records.insert("poison".to_string(), record);
        MapContent { records }
    }

    #[test]
    fn replace_overwrites_existing() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let svc = StatusEffectService::new(
            StatusEffectRepository::new(&db),
            content("replace", 5),
            Some(FixedClock(2)),
        );
        svc.apply("hero", "poison", None, None, None).unwrap();
        let second = svc.apply("hero", "poison", None, None, None).unwrap();
        let listed = svc.list_for_entity("hero").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, second.id);
        assert_eq!(second.expires_at_tick, Some(7));
    }

    #[test]
    fn extend_sums_durations() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let svc = StatusEffectService::new(
            StatusEffectRepository::new(&db),
            content("extend", 5),
            Some(FixedClock(0)),
        );
        svc.apply("hero", "poison", None, None, None).unwrap();
        svc.apply("hero", "poison", Some(3), None, None).unwrap();
        let listed = svc.list_for_entity("hero").unwrap();
        assert_eq!(listed.len(), 1);
        // existing expiry 5, max(5, 0) + 3 = 8
        assert_eq!(listed[0].expires_at_tick, Some(8));
    }

    #[test]
    fn stack_keeps_parallel_instances_and_tags() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let svc = StatusEffectService::new(
            StatusEffectRepository::new(&db),
            content("stack", 0),
            Some(FixedClock(0)),
        );
        svc.apply("hero", "poison", None, None, None).unwrap();
        svc.apply("hero", "poison", None, None, None).unwrap();
        assert_eq!(svc.list_for_entity("hero").unwrap().len(), 2);
        let tags = svc.get_tags("hero").unwrap();
        assert!(tags.contains("poisoned"));
        assert_eq!(svc.remove("hero", "poison").unwrap(), 2);
    }

    #[test]
    fn missing_effect_is_error() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let svc: StatusEffectService<'_, MapContent, FixedClock> = StatusEffectService::new(
            StatusEffectRepository::new(&db),
            MapContent {
                records: HashMap::new(),
            },
            None,
        );
        assert!(svc.apply("hero", "ghost", None, None, None).is_err());
    }
}
