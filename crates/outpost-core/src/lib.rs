//! Outpost Core: simulation, domain, and event-sourcing friendly API (desktop/WASM-agnostic)

use serde::{Deserialize, Serialize};

pub mod hex;
pub use hex::Hex;

pub type Result<T, E = CoreError> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not implemented")] 
    NotImplemented,
}

// --- Domain snapshots (minimal for scaffolding) ---

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct GameState {
    pub turn: u64,
    pub population_total: i64,
    pub population_employed: i64,
    pub power_generation: i64,
    pub power_consumption: i64,
    pub resources_stock: i64,
    /// Bitset of enabled layers (see Layer::bit). Scaffold for UI toggles.
    pub layers_enabled: u8,
}

// --- Grid/Hex validation parameters (simple scaffold) ---
/// Inclusive axial coordinate bounds for quick validation during scaffolding.
/// These are not final gameplay limits; client can render any span, but core
/// uses these to keep property tests and examples bounded and deterministic.
pub const GRID_Q_MIN: i32 = -100;
pub const GRID_Q_MAX: i32 = 100;
pub const GRID_R_MIN: i32 = -100;
pub const GRID_R_MAX: i32 = 100;

#[inline]
pub fn is_hex_in_bounds(q: i32, r: i32) -> bool {
    (GRID_Q_MIN..=GRID_Q_MAX).contains(&q) && (GRID_R_MIN..=GRID_R_MAX).contains(&r)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    AdvanceTurn,
    PlaceBuilding { q: i32, r: i32, building_type: String },
    ToggleLayer { layer: Layer },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    TurnAdvanced { new_turn: u64 },
    BuildingPlaced { q: i32, r: i32, building_type: String },
    LayerToggled { layer: Layer },
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Layer {
    Power,
    Resources,
    Pollution,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdvanceTurnResult {
    pub power_generation: i64,
    pub power_consumption: i64,
    pub resources_delta: i64,
}

// --- Core API ---

pub fn handle(state: &GameState, cmd: Command) -> Result<Vec<Event>> {
    match cmd {
        Command::AdvanceTurn => Ok(vec![Event::TurnAdvanced { new_turn: state.turn + 1 }]),
        Command::PlaceBuilding { q, r, building_type } => {
            // Minimal validation scaffold
            if building_type.is_empty() {
                return Err(CoreError::Validation("building_type required".into()));
            }
            if !is_hex_in_bounds(q, r) {
                return Err(CoreError::Validation(format!(
                    "hex out of bounds: q={}, r={} (bounds q:{}..{}, r:{}..{})",
                    q, r, GRID_Q_MIN, GRID_Q_MAX, GRID_R_MIN, GRID_R_MAX
                )));
            }
            Ok(vec![Event::BuildingPlaced { q, r, building_type }])
        }
        Command::ToggleLayer { layer } => Ok(vec![Event::LayerToggled { layer }]),
    }
}

pub fn apply(mut state: GameState, event: &Event) -> GameState {
    match event {
        Event::TurnAdvanced { new_turn } => {
            state.turn = *new_turn;
            // very rough placeholder simulation deltas
            let res = advance_turn(&state);
            state.power_generation = res.power_generation;
            state.power_consumption = res.power_consumption;
            state.resources_stock += res.resources_delta;
        }
        Event::BuildingPlaced { .. } => {
            // TODO: update state accordingly
        }
        Event::LayerToggled { layer } => {
            let bit = layer.bit();
            // toggle
            state.layers_enabled ^= bit;
        }
    }
    state
}

/// Skeleton turn advancement: returns placeholder deterministic deltas for early UI wiring.
pub fn advance_turn_sim(_state: &GameState) -> AdvanceTurnResult {
    // Placeholder deterministic numbers to make charts/UI wiring easy early on
    AdvanceTurnResult { power_generation: 100, power_consumption: 75, resources_delta: 5 }
}

/// Public advance_turn API as per pivot checklist; delegates to the current simulation stub.
#[inline]
pub fn advance_turn(state: &GameState) -> AdvanceTurnResult {
    advance_turn_sim(state)
}

// Convenience helpers for callers that want to execute a command and apply events
pub fn exec_and_apply(state: &GameState, cmd: Command) -> Result<GameState> {
    let events = handle(state, cmd)?;
    let mut s = state.clone();
    for e in &events {
        s = apply(s, e);
    }
    Ok(s)
}

// --- Queries (scaffold) ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PowerSnapshot { pub generation: i64, pub consumption: i64 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PopulationSnapshot { pub total: i64, pub employed: i64 }

pub fn get_power_snapshot(state: &GameState) -> PowerSnapshot {
    PowerSnapshot { generation: state.power_generation, consumption: state.power_consumption }
}

pub fn get_population_snapshot(state: &GameState) -> PopulationSnapshot {
    PopulationSnapshot { total: state.population_total, employed: state.population_employed }
}

/// Returns the resources stock series for a slice of states (e.g., a history window).
pub fn get_resource_series(states: &[GameState]) -> Vec<i64> {
    states.iter().map(|s| s.resources_stock).collect()
}

// --- Layer helpers ---

impl Layer {
    #[inline]
    pub const fn bit(self) -> u8 {
        match self {
            Layer::Power => 1 << 0,
            Layer::Resources => 1 << 1,
            Layer::Pollution => 1 << 2,
        }
    }
}

#[inline]
pub fn is_layer_enabled(state: &GameState, layer: Layer) -> bool {
    (state.layers_enabled & layer.bit()) != 0
}

#[inline]
pub fn active_layers(state: &GameState) -> Vec<Layer> {
    [Layer::Power, Layer::Resources, Layer::Pollution]
        .into_iter()
        .filter(|l| is_layer_enabled(state, *l))
        .collect()
}

// --- Serialization helpers (JSON/RON) ---

pub fn game_state_to_json(state: &GameState) -> Result<String> {
    serde_json::to_string(state).map_err(|e| CoreError::Validation(e.to_string()))
}

pub fn game_state_from_json(s: &str) -> Result<GameState> {
    serde_json::from_str(s).map_err(|e| CoreError::Validation(e.to_string()))
}

pub fn game_state_to_ron(state: &GameState) -> Result<String> {
    ron::to_string(state).map_err(|e| CoreError::Validation(e.to_string()))
}

pub fn game_state_from_ron(s: &str) -> Result<GameState> {
    ron::from_str(s).map_err(|e| CoreError::Validation(e.to_string()))
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn advance_turn_creates_event_and_applies() {
        let state = GameState::default();
        let events = handle(&state, Command::AdvanceTurn).unwrap();
        assert!(matches!(events.as_slice(), [Event::TurnAdvanced { .. }]));
        let s2 = events.iter().fold(state.clone(), |s, e| apply(s, e));
        assert_eq!(s2.turn, 1);
        let power = get_power_snapshot(&s2);
        assert!(power.generation > 0);
    }

    #[test]
    fn place_building_validation() {
        let state = GameState::default();
        // Empty building type rejected
        let err = handle(&state, Command::PlaceBuilding { q: 0, r: 0, building_type: "".into()})
            .unwrap_err();
        matches!(err, CoreError::Validation(_));

        // Out of bounds rejected
        let err2 = handle(&state, Command::PlaceBuilding { q: GRID_Q_MAX + 1, r: 0, building_type: "Solar".into()})
            .unwrap_err();
        matches!(err2, CoreError::Validation(_));

        // In bounds accepted
        let ok = handle(&state, Command::PlaceBuilding { q: 1, r: -1, building_type: "Solar".into()})
            .unwrap();
        assert!(matches!(ok.as_slice(), [Event::BuildingPlaced { .. }]));
    }

    #[test]
    fn resource_series_collects() {
        let series = vec![
            GameState { resources_stock: 1, ..Default::default() },
            GameState { resources_stock: 3, ..Default::default() },
            GameState { resources_stock: 2, ..Default::default() },
        ];
        assert_eq!(get_resource_series(&series), vec![1,3,2]);
    }

    #[test]
    fn layer_toggle_works_and_is_queryable() {
        let state = GameState::default();
        assert!(!is_layer_enabled(&state, Layer::Power));
        let s1 = exec_and_apply(&state, Command::ToggleLayer { layer: Layer::Power }).unwrap();
        assert!(is_layer_enabled(&s1, Layer::Power));
        // Toggling again disables
        let s2 = exec_and_apply(&s1, Command::ToggleLayer { layer: Layer::Power }).unwrap();
        assert!(!is_layer_enabled(&s2, Layer::Power));
        // Independent bits
        let s3 = exec_and_apply(&s2, Command::ToggleLayer { layer: Layer::Resources }).unwrap();
        assert!(is_layer_enabled(&s3, Layer::Resources));
        assert!(!is_layer_enabled(&s3, Layer::Power));
    }

    // --- Property-based tests ---
    fn arb_gamestate() -> impl Strategy<Value = GameState> {
        (
            any::<u64>(),
            -1_000i64..=1_000,
            -1_000i64..=1_000,
            -10_000i64..=10_000,
            -10_000i64..=10_000,
            -1_000_000i64..=1_000_000,
        ).prop_map(|(turn, pop_t, pop_e, pg, pc, rs)| GameState {
            turn,
            population_total: pop_t,
            population_employed: pop_e.clamp(0, pop_t.max(0)),
            power_generation: pg,
            power_consumption: pc,
            resources_stock: rs,
            layers_enabled: 0,
        })
    }

    proptest! {
        #[test]
        fn json_roundtrip(state in arb_gamestate()) {
            let s = game_state_to_json(&state).unwrap();
            let de = game_state_from_json(&s).unwrap();
            prop_assert_eq!(state, de);
        }

        #[test]
        fn ron_roundtrip(state in arb_gamestate()) {
            let s = game_state_to_ron(&state).unwrap();
            let de = game_state_from_ron(&s).unwrap();
            prop_assert_eq!(state, de);
        }

        #[test]
        fn placement_in_bounds_accepted((q, r) in (GRID_Q_MIN..=GRID_Q_MAX, GRID_R_MIN..=GRID_R_MAX), name in "[A-Za-z]{1,8}") {
            let state = GameState::default();
            let ev = handle(&state, Command::PlaceBuilding { q, r, building_type: name }).unwrap();
            let is_ok = matches!(ev.as_slice(), [Event::BuildingPlaced { .. }]);
            prop_assert!(is_ok);
        }

        #[test]
        fn placement_out_of_bounds_rejected((q, r) in (
            (GRID_Q_MAX+1..=GRID_Q_MAX+100).prop_union(GRID_Q_MIN-100..=GRID_Q_MIN-1),
            GRID_R_MIN-100..=GRID_R_MAX+100
        ), name in "[A-Za-z]{1,8}") {
            // ensure at least one of q,r out of bounds
            let state = GameState::default();
            let res = handle(&state, Command::PlaceBuilding { q, r, building_type: name });
            prop_assert!(res.is_err());
        }

        #[test]
        fn toggling_parity_matches_bitset(seq in prop::collection::vec(prop_oneof![Just(Layer::Power), Just(Layer::Resources), Just(Layer::Pollution)], 0..64)) {
            let mut state = GameState::default();
            let mut counts = std::collections::HashMap::<Layer, i32>::new();
            for l in seq {
                *counts.entry(l).or_insert(0) += 1;
                state = exec_and_apply(&state, Command::ToggleLayer { layer: l }).unwrap();
            }
            for l in [Layer::Power, Layer::Resources, Layer::Pollution] {
                let parity = counts.get(&l).copied().unwrap_or(0) % 2 != 0;
                prop_assert_eq!(is_layer_enabled(&state, l), parity);
            }
        }
    }
}
