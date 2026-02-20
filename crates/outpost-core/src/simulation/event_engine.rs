//! Game Event Engine — MVP.6.1
//!
//! Evaluates trigger conditions against game state each tick, fires gameplay events
//! (distinct from event-sourcing `GameEvent`), applies outcomes, and queues player choices.
//!
//! Design rules:
//! - Pure: no I/O, no side effects beyond mutating `GameState` fields.
//! - Deterministic given the same seed (RNG seeded from tick + site_id).
//! - All trigger variants must be exhaustively matched.

use crate::content::{EventDefinition, EventTrigger, EventChoice, EventOutcome};
use crate::domain::game_state::PendingEventChoice;
use crate::domain::resource_mapping::string_to_resource_type;
use crate::domain::{GameState, SiteId};
use crate::events::{EventType, GameEvent};

// ─────────────────────────────────────────────────────────────────────────────
// Trigger evaluation
// ─────────────────────────────────────────────────────────────────────────────

/// Evaluate a single trigger condition against site state.
/// Returns `true` if the condition is satisfied (excluding random rolls).
pub fn evaluate_trigger(trigger: &EventTrigger, site: &crate::domain::Site, tick: u64) -> bool {
    match trigger {
        EventTrigger::BuildingConstructed { building_id } => {
            site.buildings.values().any(|b| {
                &b.building_def_id == building_id
                    && b.state.can_operate()
            })
        }
        EventTrigger::PopulationReached { threshold } => {
            site.population.total >= (*threshold as u64)
        }
        EventTrigger::ResourceThreshold { resource_id, amount, above } => {
            match string_to_resource_type(resource_id) {
                Ok(rt) => {
                    let stock = site.resources.get(rt) as f64;
                    if *above { stock >= *amount } else { stock < *amount }
                }
                Err(_) => false,
            }
        }
        EventTrigger::TimeBased { ticks } => {
            tick >= *ticks
        }
        EventTrigger::MoraleThreshold { threshold } => {
            site.population.morale <= (*threshold as f32)
        }
        // Random triggers are handled separately (probability roll)
        EventTrigger::Random { .. } => true,
    }
}

/// Return the probability of firing this tick, combining all `Random` triggers.
/// If no `Random` trigger exists, probability is 1.0 (fires deterministically).
fn event_probability(def: &EventDefinition) -> f64 {
    let random_triggers: Vec<f64> = def.triggers.iter()
        .filter_map(|t| {
            if let EventTrigger::Random { probability_per_tick } = t {
                Some(*probability_per_tick)
            } else {
                None
            }
        })
        .collect();

    if random_triggers.is_empty() {
        1.0
    } else {
        // Combined probability: product of independent random triggers
        random_triggers.iter().copied().fold(1.0, |acc, p| acc * p)
    }
}

/// Deterministic hash-based random number in [0, 1) from a seed.
/// Avoids pulling in a heavy RNG crate while remaining seedable.
fn pseudo_random(seed: u64) -> f64 {
    // Wang hash
    let mut x = seed ^ (seed << 13);
    x ^= x >> 7;
    x ^= x << 17;
    x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (x >> 33) as f64 / (u32::MAX as f64)
}

/// Decide whether this event fires this tick.
///
/// All non-random triggers must be satisfied AND the probability roll must pass.
pub fn should_fire(
    def: &EventDefinition,
    site: &crate::domain::Site,
    tick: u64,
    seed: u64,
) -> bool {
    // All non-random triggers must pass
    let conditions_met = def.triggers.iter().all(|t| evaluate_trigger(t, site, tick));
    if !conditions_met {
        return false;
    }

    let prob = event_probability(def);
    if prob >= 1.0 {
        return true;
    }

    pseudo_random(seed) < prob
}

// ─────────────────────────────────────────────────────────────────────────────
// Outcome application
// ─────────────────────────────────────────────────────────────────────────────

/// Apply a single `EventOutcome` to a site.  Returns a short log string for debugging.
pub fn apply_outcome(site: &mut crate::domain::Site, outcome: &EventOutcome) {
    match outcome {
        EventOutcome::ResourceChange { resource_id, amount } => {
            if let Ok(rt) = string_to_resource_type(resource_id) {
                let delta = *amount as i64;
                site.resources.add_resource(rt, delta);
            }
        }
        EventOutcome::MoraleChange { amount } => {
            site.population.morale = (site.population.morale + *amount as f32)
                .clamp(0.0, 100.0);
        }
        EventOutcome::PopulationChange { amount } => {
            let new_pop = (site.population.total as i64 + *amount as i64).max(0) as u64;
            site.population.total = new_pop;
        }
        // These require server-side handling (unlocks, chained events, narrative) —
        // we emit them as events but do not apply state directly in the engine.
        EventOutcome::UnlockBuilding { .. }
        | EventOutcome::TriggerEvent { .. }
        | EventOutcome::NarrativeText { .. } => {}
    }
}

/// Apply all outcomes of a chosen `EventChoice` to the site.
pub fn apply_choice(site: &mut crate::domain::Site, choice: &EventChoice) {
    for outcome in &choice.outcomes {
        apply_outcome(site, outcome);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-tick processing
// ─────────────────────────────────────────────────────────────────────────────

/// Process all event definitions for every site in a single tick.
///
/// Returns the list of `GameEvent`s emitted (for the event log / event sourcing store).
/// Also mutates `state.event_engine` (cooldowns, fired set, pending choices) and
/// optionally `state.clock` (auto-pause on critical events).
pub fn process_events_tick(state: &mut GameState) -> Vec<GameEvent> {
    let tick = state.current_tick();
    let defs: Vec<EventDefinition> = state.event_definitions.values().cloned().collect();
    let site_ids: Vec<SiteId> = state.galaxy.all_sites().iter().map(|s| s.id).collect();

    let mut emitted: Vec<GameEvent> = Vec::new();

    for site_id in &site_ids {
        let site_id_str = site_id.to_string();

        for def in &defs {
            let key = format!("{}:{}", site_id_str, def.id);

            // Skip non-repeatable events that have already fired
            if !def.repeatable && state.event_engine.fired_non_repeatable.contains(&key) {
                continue;
            }

            // Skip events still in cooldown
            if let Some(&until) = state.event_engine.cooldown_until.get(&key) {
                if tick < until {
                    continue;
                }
            }

            // Find the site and check triggers
            let site = match state.galaxy.find_site(site_id) {
                Some(s) => s,
                None => continue,
            };

            // Seed: mix tick, site UUID low bits, and event id hash
            let event_hash = def.id.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
            let seed = tick.wrapping_mul(0x517cc1b727220a95)
                .wrapping_add(site_id.0.as_u128() as u64)
                .wrapping_add(event_hash);

            if !should_fire(def, site, tick, seed) {
                continue;
            }

            // ── Event fires ──

            let requires_choice = def.choices.len() > 1;
            let auto_resolve = def.choices.len() == 1;

            emitted.push(GameEvent::new(
                1,
                tick,
                EventType::GameEventFired {
                    site_id: *site_id,
                    event_id: def.id.clone(),
                    title: def.title.clone(),
                    tick,
                    requires_choice,
                },
            ));

            // Auto-resolve single-choice events immediately
            if auto_resolve {
                let choice = def.choices[0].clone();
                // Apply outcomes
                if let Some(site_mut) = state.galaxy.find_site_mut(site_id) {
                    apply_choice(site_mut, &choice);
                }
                emitted.push(GameEvent::new(
                    1,
                    tick,
                    EventType::GameEventChoiceResolved {
                        site_id: *site_id,
                        event_id: def.id.clone(),
                        choice_id: choice.id.clone(),
                        tick,
                    },
                ));
            } else if requires_choice {
                // Queue the pending choice for player resolution
                state.event_engine.pending_choices.push(PendingEventChoice {
                    site_id: site_id_str.clone(),
                    event_id: def.id.clone(),
                    fired_at_tick: tick,
                    title: def.title.clone(),
                    description: def.description.clone(),
                });
            }

            // Record cooldown / fired-once
            if !def.repeatable {
                state.event_engine.fired_non_repeatable.insert(key.clone());
            } else if let Some(cd) = def.cooldown_ticks {
                state.event_engine.cooldown_until.insert(key, tick + cd);
            }

            // Auto-pause on critical events
            if def.auto_pause || matches!(def.severity, crate::content::EventSeverity::Critical) {
                state.pause();
            }
        }
    }

    emitted
}

/// Resolve a pending choice for a site/event and apply outcomes.
///
/// Returns `Some(GameEvent)` if the choice was found and resolved, `None` otherwise.
pub fn resolve_choice(
    state: &mut GameState,
    site_id: SiteId,
    event_id: &str,
    choice_id: &str,
) -> Option<GameEvent> {
    let tick = state.current_tick();

    // Remove from pending list
    let pending_idx = state.event_engine.pending_choices
        .iter()
        .position(|p| p.site_id == site_id.to_string() && p.event_id == event_id);
    if let Some(idx) = pending_idx {
        state.event_engine.pending_choices.remove(idx);
    }

    // Find the choice
    let choice = state.event_definitions.get(event_id)
        ?.choices.iter()
        .find(|c| c.id == choice_id)
        .cloned()?;

    // Apply outcomes
    if let Some(site) = state.galaxy.find_site_mut(&site_id) {
        apply_choice(site, &choice);
    }

    Some(GameEvent::new(
        1,
        tick,
        EventType::GameEventChoiceResolved {
            site_id,
            event_id: event_id.to_string(),
            choice_id: choice_id.to_string(),
            tick,
        },
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{EventCategory, EventSeverity, EventTrigger, EventChoice, EventOutcome};
    use crate::domain::{CelestialBodyId, GameState, Site};
    use crate::domain::star_system::StarSystem;
    use crate::domain::galaxy::Galaxy;
    use std::collections::HashMap;

    fn make_site(pop: u64) -> Site {
        let mut site = Site::new_settlement(CelestialBodyId::new(), "Test Base".into(), 0, pop as u32);
        site.population.total = pop;
        site
    }

    fn make_event(id: &str, triggers: Vec<EventTrigger>, choices: Vec<EventChoice>, repeatable: bool) -> EventDefinition {
        EventDefinition {
            id: id.to_string(),
            title: id.to_string(),
            description: "test event".to_string(),
            category: EventCategory::General,
            severity: EventSeverity::Info,
            auto_pause: false,
            triggers,
            choices,
            repeatable,
            cooldown_ticks: None,
            weight: 1.0,
        }
    }

    fn single_choice(id: &str, outcomes: Vec<EventOutcome>) -> EventChoice {
        EventChoice {
            id: id.to_string(),
            text: id.to_string(),
            description: String::new(),
            outcomes,
            cost: HashMap::new(),
        }
    }

    fn make_game_state_with_site(site: Site) -> (GameState, SiteId) {
        let mut gs = GameState::new("Test".into(), 1);
        let site_id = site.id;
        let mut system = StarSystem::new(gs.galaxy.id, "Sys".into(), 0);
        system.add_site(site);
        gs.galaxy.add_system(system);
        (gs, site_id)
    }

    // ── Trigger evaluation ──

    #[test]
    fn test_trigger_population_reached_passes() {
        let site = make_site(100);
        let trigger = EventTrigger::PopulationReached { threshold: 50 };
        assert!(evaluate_trigger(&trigger, &site, 1));
    }

    #[test]
    fn test_trigger_population_reached_fails() {
        let site = make_site(10);
        let trigger = EventTrigger::PopulationReached { threshold: 50 };
        assert!(!evaluate_trigger(&trigger, &site, 1));
    }

    #[test]
    fn test_trigger_morale_threshold_passes() {
        let mut site = make_site(50);
        site.population.morale = 20.0;
        let trigger = EventTrigger::MoraleThreshold { threshold: 30.0 };
        assert!(evaluate_trigger(&trigger, &site, 1));
    }

    #[test]
    fn test_trigger_morale_threshold_fails_when_high() {
        let mut site = make_site(50);
        site.population.morale = 80.0;
        let trigger = EventTrigger::MoraleThreshold { threshold: 30.0 };
        assert!(!evaluate_trigger(&trigger, &site, 1));
    }

    #[test]
    fn test_trigger_time_based_passes() {
        let site = make_site(10);
        let trigger = EventTrigger::TimeBased { ticks: 100 };
        assert!(evaluate_trigger(&trigger, &site, 100));
        assert!(evaluate_trigger(&trigger, &site, 200));
    }

    #[test]
    fn test_trigger_time_based_fails_before() {
        let site = make_site(10);
        let trigger = EventTrigger::TimeBased { ticks: 100 };
        assert!(!evaluate_trigger(&trigger, &site, 99));
    }

    #[test]
    fn test_trigger_building_constructed_passes() {
        use crate::domain::building_instance::{BuildingInstance, BuildingState};
        let mut site = make_site(10);
        let mut building = BuildingInstance::new_operational(site.id, "mine".into(), 0);
        building.state = BuildingState::Operational;
        site.buildings.insert(building.id, building);

        let trigger = EventTrigger::BuildingConstructed { building_id: "mine".into() };
        assert!(evaluate_trigger(&trigger, &site, 1));
    }

    #[test]
    fn test_trigger_building_constructed_fails_no_building() {
        let site = make_site(10);
        let trigger = EventTrigger::BuildingConstructed { building_id: "mine".into() };
        assert!(!evaluate_trigger(&trigger, &site, 1));
    }

    #[test]
    fn test_trigger_resource_threshold_above() {
        use crate::domain::ResourceType;
        let mut site = make_site(10);
        site.resources.set(ResourceType::IronOre, 100);
        let trigger = EventTrigger::ResourceThreshold {
            resource_id: "iron_ore".into(),
            amount: 50.0,
            above: true,
        };
        assert!(evaluate_trigger(&trigger, &site, 1));
    }

    #[test]
    fn test_trigger_resource_threshold_below() {
        use crate::domain::ResourceType;
        let mut site = make_site(10);
        site.resources.set(ResourceType::IronOre, 5);
        let trigger = EventTrigger::ResourceThreshold {
            resource_id: "iron_ore".into(),
            amount: 50.0,
            above: false,
        };
        assert!(evaluate_trigger(&trigger, &site, 1));
    }

    // ── Probability rolling ──

    #[test]
    fn test_deterministic_random_is_consistent() {
        let a = pseudo_random(12345);
        let b = pseudo_random(12345);
        assert_eq!(a, b, "pseudo_random must be deterministic");
    }

    #[test]
    fn test_event_fires_with_prob_1() {
        let site = make_site(100);
        let def = make_event(
            "guaranteed",
            vec![EventTrigger::PopulationReached { threshold: 1 }],
            vec![single_choice("ok", vec![])],
            false,
        );
        // With probability 1.0, should always fire
        assert!(should_fire(&def, &site, 1, 0));
        assert!(should_fire(&def, &site, 1, 999999));
    }

    #[test]
    fn test_event_with_zero_prob_never_fires() {
        let site = make_site(100);
        let def = make_event(
            "impossible",
            vec![
                EventTrigger::PopulationReached { threshold: 1 },
                EventTrigger::Random { probability_per_tick: 0.0 },
            ],
            vec![single_choice("ok", vec![])],
            false,
        );
        // 0.0 probability means never
        for seed in 0..1000u64 {
            assert!(!should_fire(&def, &site, 1, seed));
        }
    }

    #[test]
    fn test_event_does_not_fire_when_trigger_fails() {
        let site = make_site(1); // pop=1, threshold=100 fails
        let def = make_event(
            "pop_event",
            vec![EventTrigger::PopulationReached { threshold: 100 }],
            vec![single_choice("ok", vec![])],
            false,
        );
        assert!(!should_fire(&def, &site, 1, 0));
    }

    // ── Outcome application ──

    #[test]
    fn test_apply_morale_change_clamps_to_100() {
        let mut site = make_site(10);
        site.population.morale = 95.0;
        apply_outcome(&mut site, &EventOutcome::MoraleChange { amount: 20.0 });
        assert_eq!(site.population.morale, 100.0);
    }

    #[test]
    fn test_apply_morale_change_clamps_to_0() {
        let mut site = make_site(10);
        site.population.morale = 5.0;
        apply_outcome(&mut site, &EventOutcome::MoraleChange { amount: -50.0 });
        assert_eq!(site.population.morale, 0.0);
    }

    #[test]
    fn test_apply_population_change_no_negative() {
        let mut site = make_site(5);
        apply_outcome(&mut site, &EventOutcome::PopulationChange { amount: -100 });
        assert_eq!(site.population.total, 0);
    }

    #[test]
    fn test_apply_resource_change() {
        use crate::domain::ResourceType;
        let mut site = make_site(10);
        site.resources.set(ResourceType::IronOre, 50);
        apply_outcome(&mut site, &EventOutcome::ResourceChange {
            resource_id: "iron_ore".into(),
            amount: 30.0,
        });
        assert_eq!(site.resources.get(ResourceType::IronOre), 80);
    }

    // ── Process events tick ──

    #[test]
    fn test_non_repeatable_event_fires_once() {
        let site = make_site(100);
        let (mut gs, _) = make_game_state_with_site(site);

        let def = make_event(
            "once_event",
            vec![EventTrigger::PopulationReached { threshold: 1 }],
            vec![single_choice("ok", vec![])],
            false, // non-repeatable
        );
        gs.event_definitions.insert(def.id.clone(), def);

        let events1 = process_events_tick(&mut gs);
        gs.advance_tick();
        let events2 = process_events_tick(&mut gs);

        let fired1 = events1.iter().filter(|e| matches!(&e.event_type, EventType::GameEventFired { .. })).count();
        let fired2 = events2.iter().filter(|e| matches!(&e.event_type, EventType::GameEventFired { .. })).count();

        assert_eq!(fired1, 1, "Event must fire on first tick");
        assert_eq!(fired2, 0, "Non-repeatable event must not fire again");
    }

    #[test]
    fn test_cooldown_prevents_immediate_refire() {
        let site = make_site(100);
        let (mut gs, _) = make_game_state_with_site(site);

        let mut def = make_event(
            "repeat_event",
            vec![EventTrigger::PopulationReached { threshold: 1 }],
            vec![single_choice("ok", vec![])],
            true, // repeatable
        );
        def.cooldown_ticks = Some(10);
        gs.event_definitions.insert(def.id.clone(), def);

        let events1 = process_events_tick(&mut gs);
        gs.advance_tick(); // tick 1
        let events2 = process_events_tick(&mut gs);

        let fired1 = events1.iter().filter(|e| matches!(&e.event_type, EventType::GameEventFired { .. })).count();
        let fired2 = events2.iter().filter(|e| matches!(&e.event_type, EventType::GameEventFired { .. })).count();

        assert_eq!(fired1, 1, "Repeatable event fires on first tick");
        assert_eq!(fired2, 0, "Cooldown must prevent refire on next tick");
    }

    #[test]
    fn test_single_choice_auto_resolved() {
        let site = make_site(100);
        let (mut gs, site_id) = make_game_state_with_site(site);

        let def = make_event(
            "auto_resolve",
            vec![EventTrigger::PopulationReached { threshold: 1 }],
            vec![single_choice("celebrate", vec![
                EventOutcome::MoraleChange { amount: 10.0 },
            ])],
            false,
        );
        gs.event_definitions.insert(def.id.clone(), def);

        // Record morale before
        let morale_before = gs.galaxy.find_site(&site_id).unwrap().population.morale;
        let events = process_events_tick(&mut gs);

        // Check GameEventChoiceResolved was emitted
        let resolved = events.iter().any(|e| matches!(&e.event_type, EventType::GameEventChoiceResolved { .. }));
        assert!(resolved, "Single-choice event must auto-resolve");

        // Check morale improved
        let morale_after = gs.galaxy.find_site(&site_id).unwrap().population.morale;
        assert!(morale_after > morale_before, "Morale must have increased after auto-resolve");
    }

    #[test]
    fn test_multi_choice_event_queued_as_pending() {
        let site = make_site(100);
        let (mut gs, site_id) = make_game_state_with_site(site);

        let mut def = make_event(
            "choice_event",
            vec![EventTrigger::PopulationReached { threshold: 1 }],
            vec![
                single_choice("option_a", vec![EventOutcome::MoraleChange { amount: 5.0 }]),
                single_choice("option_b", vec![EventOutcome::MoraleChange { amount: -5.0 }]),
            ],
            false,
        );
        def.choices[1].id = "option_b".to_string();
        gs.event_definitions.insert(def.id.clone(), def);

        process_events_tick(&mut gs);

        let pending = gs.event_engine.pending_choices.iter()
            .any(|p| p.event_id == "choice_event" && p.site_id == site_id.to_string());
        assert!(pending, "Multi-choice event must be queued as pending");
    }

    #[test]
    fn test_auto_pause_on_critical_severity() {
        let site = make_site(100);
        let (mut gs, _) = make_game_state_with_site(site);

        let mut def = make_event(
            "critical_event",
            vec![EventTrigger::PopulationReached { threshold: 1 }],
            vec![single_choice("ok", vec![])],
            false,
        );
        def.severity = EventSeverity::Critical;
        gs.event_definitions.insert(def.id.clone(), def);

        assert!(!gs.is_paused());
        process_events_tick(&mut gs);
        assert!(gs.is_paused(), "Critical event must trigger auto-pause");
    }

    // ── Property tests ──

    #[cfg(feature = "proptest")]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn morale_always_in_bounds_after_outcome(delta in -200.0f64..200.0) {
                let mut site = make_site(50);
                site.population.morale = 50.0;
                apply_outcome(&mut site, &EventOutcome::MoraleChange { amount: delta });
                prop_assert!(site.population.morale >= 0.0);
                prop_assert!(site.population.morale <= 100.0);
            }

            #[test]
            fn population_never_negative_after_outcome(delta in -10000i32..10000) {
                let mut site = make_site(50);
                apply_outcome(&mut site, &EventOutcome::PopulationChange { amount: delta });
                prop_assert!(site.population.total < i64::MAX as u64);
            }
        }
    }
}
