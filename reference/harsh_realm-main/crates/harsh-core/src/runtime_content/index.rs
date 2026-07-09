//! Trigger sourcing — which of an entity's triggers fire for a given event.
//!
//! Pure and DB-free so it is unit/property-testable in isolation. Triggers are
//! *not* global: they are owned by records an entity currently carries. For an
//! event `E` on an entity, the active trigger set is, in stable order:
//!
//! 1. the entity's active **status effects'** `triggers`,
//! 2. the entity's **equipped items'** `grants_traits` → each trait's `triggers`,
//! 3. the entity's **intrinsic traits'** `triggers`,
//! 4. **standalone/global `trigger` records** in the store (rooms/objects/world
//!    reactions) that subscribe to `E` regardless of who is acting,
//!
//! each filtered to `trigger.on == E`. Pass intrinsic traits separately from
//! item-granted traits so the two sources are not double-counted. Global triggers
//! come last, so an entity's own reactions resolve before world reactions.

use crate::ir::Trigger;

use super::store::RuntimeContentStore;

/// Collect, in source order, the triggers on `event_type` that an entity carries
/// via its active statuses, equipped items' granted traits, and intrinsic traits.
pub fn triggers_for_event<'s>(
    store: &'s RuntimeContentStore,
    event_type: &str,
    active_status_ids: &[String],
    equipped_item_ids: &[String],
    intrinsic_trait_ids: &[String],
) -> Vec<&'s Trigger> {
    let mut out: Vec<&Trigger> = Vec::new();

    // 1. active status effects' triggers
    for status_id in active_status_ids {
        if let Some(effect) = store.get_status_effect(status_id) {
            push_matching(&mut out, &effect.triggers, event_type);
        }
    }
    // 2. equipped items' granted traits' triggers
    for item_id in equipped_item_ids {
        if let Some(item) = store.get_item(item_id) {
            for trait_id in &item.grants_traits {
                if let Some(tr) = store.get_trait(trait_id) {
                    push_matching(&mut out, &tr.triggers, event_type);
                }
            }
        }
    }
    // 3. intrinsic traits' triggers
    for trait_id in intrinsic_trait_ids {
        if let Some(tr) = store.get_trait(trait_id) {
            push_matching(&mut out, &tr.triggers, event_type);
        }
    }
    // 4. standalone/global trigger records (already filtered to event_type)
    out.extend(store.standalone_triggers_on(event_type));

    out
}

fn push_matching<'s>(out: &mut Vec<&'s Trigger>, triggers: &'s [Trigger], event_type: &str) {
    for t in triggers {
        if t.on == event_type {
            out.push(t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::ComponentRecord;
    use serde_json::{json, Value};

    fn store() -> RuntimeContentStore {
        let records: Vec<ComponentRecord> = vec![
            json!({
                "component_type": "status_effect", "id": "poisoned", "name": "Poisoned",
                "provides_tags": ["poisoned"],
                "triggers": [{"id": "poison_tick", "on": "combat.attack",
                              "when": "has_tag(self, 'poisoned')",
                              "do": [{"kind": "change_resource",
                                      "params": {"resource": "hp", "delta": -2}}]}]
            }),
            json!({
                "component_type": "trait", "id": "envenom_on_hit", "name": "Envenom on Hit",
                "category": "weapon_quality",
                "triggers": [{"id": "envenom_strike", "on": "combat.attack",
                              "when": "event.hit == true",
                              "do": [{"kind": "apply_status",
                                      "params": {"entity_id": "target", "status_id": "poisoned"}}]}]
            }),
            json!({
                "component_type": "item", "id": "envenomed_dagger", "name": "Envenomed Dagger",
                "damage": "1d4", "grants_traits": ["envenom_on_hit"]
            }),
        ]
        .into_iter()
        .map(|v: Value| serde_json::from_value(v).unwrap())
        .collect();
        RuntimeContentStore::from_records(records)
    }

    /// Store with an entity-carried status trigger plus two standalone/global
    /// triggers (to pin ordering: carried first, then globals by id).
    fn store_with_globals() -> RuntimeContentStore {
        let records: Vec<ComponentRecord> = vec![
            json!({
                "component_type": "status_effect", "id": "poisoned", "name": "Poisoned",
                "provides_tags": ["poisoned"],
                "triggers": [{"id": "poison_tick", "on": "combat.attack",
                              "when": "true", "do": []}]
            }),
            json!({
                "component_type": "trigger", "id": "b_world_alarm", "on": "combat.attack",
                "when": "true",
                "do": [{"kind": "log", "params": {"message": "the ruins shudder"}}]
            }),
            json!({
                "component_type": "trigger", "id": "a_ward_pulse", "on": "combat.attack",
                "when": "true",
                "do": [{"kind": "log", "params": {"message": "a ward pulses"}}]
            }),
            json!({
                "component_type": "trigger", "id": "elsewhere", "on": "combat.miss",
                "when": "true", "do": []
            }),
        ]
        .into_iter()
        .map(|v: Value| serde_json::from_value(v).unwrap())
        .collect();
        RuntimeContentStore::from_records(records)
    }

    #[test]
    fn orders_status_then_item_granted_trait() {
        let store = store();
        let triggers = triggers_for_event(
            &store,
            "combat.attack",
            &["poisoned".into()],
            &["envenomed_dagger".into()],
            &[],
        );
        let ids: Vec<&str> = triggers.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["poison_tick", "envenom_strike"]);
    }

    #[test]
    fn filters_by_event_type() {
        let store = store();
        let triggers = triggers_for_event(
            &store,
            "combat.miss",
            &["poisoned".into()],
            &["envenomed_dagger".into()],
            &[],
        );
        assert!(triggers.is_empty());
    }

    #[test]
    fn unknown_ids_are_ignored() {
        let store = store();
        let triggers = triggers_for_event(
            &store,
            "combat.attack",
            &["nope".into()],
            &["also_nope".into()],
            &["missing".into()],
        );
        assert!(triggers.is_empty());
    }

    #[test]
    fn standalone_globals_fire_last_and_sorted_by_id() {
        let store = store_with_globals();
        // Entity carries only the `poisoned` status; the globals fire regardless.
        let triggers = triggers_for_event(
            &store,
            "combat.attack",
            &["poisoned".into()],
            &[],
            &[],
        );
        let ids: Vec<&str> = triggers.iter().map(|t| t.id.as_str()).collect();
        // Carried trigger first; then globals on this event, sorted by id.
        assert_eq!(ids, vec!["poison_tick", "a_ward_pulse", "b_world_alarm"]);
    }

    #[test]
    fn standalone_globals_filter_by_event_type() {
        let store = store_with_globals();
        // No entity sources at all; only the global on this event fires.
        let triggers = triggers_for_event(&store, "combat.miss", &[], &[], &[]);
        let ids: Vec<&str> = triggers.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["elsewhere"]);
    }

    #[test]
    fn intrinsic_trait_triggers_included() {
        let store = store();
        let triggers =
            triggers_for_event(&store, "combat.attack", &[], &[], &["envenom_on_hit".into()]);
        let ids: Vec<&str> = triggers.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["envenom_strike"]);
    }
}
