//! Condition/predicate language — shared substrate for interrupts and directives.
//!
//! One predicate evaluator powers both "stop me when X" (interrupts, §12A) and
//! "auto-handle when X" (directives, §12). Build once; both systems consume it.
//!
//! # Predicate data model
//!
//! [`Predicate`] and [`Quantity`] are fully serialisable so they can be authored
//! in YAML/JSON content packs, persisted in `SQLite` snapshots, and sent over the
//! web API.
//!
//! # Trend / ETA semantics
//!
//! The [`Predicate::Declining`] variant extrapolates a **linear trend** from the
//! `delta` field of the colony's [`ColonyPool`] (net change last turn). That
//! single-turn rate is cheap to compute and avoids a costly forward-simulation.
//! The variant fires when the *projected* crossing time to reach zero (or the
//! supplied `threshold`) is ≤ `eta_turns` turns away.

use crate::colony::ColonyId;
use crate::turn::GameState;

/// A path into [`GameState`] that resolves to a single `f32` value.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Quantity {
    /// Stability of the named colony, in `[0.0, 1.0]`.
    ColonyStability {
        /// Target colony.
        colony_id: ColonyId,
    },
    /// Current stockpile amount for a commodity in the named colony.
    Stockpile {
        /// Target colony.
        colony_id: ColonyId,
        /// Commodity identifier (e.g. `"food"`, `"water"`).
        commodity_id: String,
    },
    /// Net per-turn rate of change for a commodity stockpile (positive = growing).
    StockpileRate {
        /// Target colony.
        colony_id: ColonyId,
        /// Commodity identifier.
        commodity_id: String,
    },
    /// Current head-count (population) of the named colony.
    ColonyPopulation {
        /// Target colony.
        colony_id: ColonyId,
    },
}

/// A boolean predicate that can be evaluated against live [`GameState`].
///
/// Predicates are pure data and are fully serialisable (YAML/JSON) so they can
/// be authored in content packs and persisted between sessions.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Predicate {
    /// True when `quantity < threshold`.
    LessThan {
        /// Left-hand quantity path.
        quantity: Quantity,
        /// Right-hand scalar threshold.
        threshold: f32,
    },
    /// True when `quantity > threshold`.
    GreaterThan {
        /// Left-hand quantity path.
        quantity: Quantity,
        /// Right-hand scalar threshold.
        threshold: f32,
    },
    /// True when both sub-predicates are true.
    And {
        /// Left-hand sub-predicate.
        left: Box<Predicate>,
        /// Right-hand sub-predicate.
        right: Box<Predicate>,
    },
    /// True when either sub-predicate is true.
    Or {
        /// Left-hand sub-predicate.
        left: Box<Predicate>,
        /// Right-hand sub-predicate.
        right: Box<Predicate>,
    },
    /// True when the inner predicate is false.
    Not {
        /// Inner predicate to negate.
        inner: Box<Predicate>,
    },
    /// True when `quantity` is on a declining trajectory that will cross
    /// `threshold` within `eta_turns` turns.
    ///
    /// Uses the colony pool's single-turn delta as the linear rate of change.
    /// Returns `false` when the rate is non-negative (quantity not declining).
    Declining {
        /// Commodity stockpile to track.
        commodity_id: String,
        /// Colony that owns the stockpile.
        colony_id: ColonyId,
        /// Value to extrapolate toward; defaults to `0.0` if this is the floor.
        threshold: f32,
        /// Maximum turns until the projected crossing for the predicate to fire.
        eta_turns: f32,
    },
}

// ─── Evaluator ───────────────────────────────────────────────────────────────

/// Stateless predicate evaluator.
///
/// All methods take `&GameState` and are pure (no side effects, no I/O).
pub struct PredicateEvaluator;

impl PredicateEvaluator {
    /// Resolve a [`Quantity`] path to a scalar value.
    ///
    /// Returns `None` when the referenced entity (colony, commodity) does not
    /// exist in the current state.
    #[must_use]
    pub fn resolve(quantity: &Quantity, state: &GameState) -> Option<f32> {
        match quantity {
            Quantity::ColonyStability { colony_id } => {
                let idx = colony_index(state, *colony_id)?;
                Some(state.populations[idx].stability)
            }
            Quantity::Stockpile {
                colony_id,
                commodity_id,
            } => {
                let idx = colony_index(state, *colony_id)?;
                #[allow(clippy::cast_possible_truncation)]
                Some(state.colonies[idx].pool.amount(commodity_id) as f32)
            }
            Quantity::StockpileRate {
                colony_id,
                commodity_id,
            } => {
                let idx = colony_index(state, *colony_id)?;
                #[allow(clippy::cast_possible_truncation)]
                Some(state.colonies[idx].pool.delta(commodity_id) as f32)
            }
            Quantity::ColonyPopulation { colony_id } => {
                let idx = colony_index(state, *colony_id)?;
                Some(state.populations[idx].count)
            }
        }
    }

    /// Evaluate a [`Predicate`] against the supplied state.
    ///
    /// Returns `false` (rather than panicking) when a referenced entity is
    /// missing — unknown colony IDs, commodity IDs that have never been
    /// deposited. This is intentional: a predicate that references a future
    /// colony will simply be inactive until that colony is founded.
    #[must_use]
    pub fn eval(predicate: &Predicate, state: &GameState) -> bool {
        match predicate {
            Predicate::LessThan {
                quantity,
                threshold,
            } => Self::resolve(quantity, state).is_some_and(|v| v < *threshold),

            Predicate::GreaterThan {
                quantity,
                threshold,
            } => Self::resolve(quantity, state).is_some_and(|v| v > *threshold),

            Predicate::And { left, right } => Self::eval(left, state) && Self::eval(right, state),

            Predicate::Or { left, right } => Self::eval(left, state) || Self::eval(right, state),

            Predicate::Not { inner } => !Self::eval(inner, state),

            Predicate::Declining {
                commodity_id,
                colony_id,
                threshold,
                eta_turns,
            } => {
                let Some(idx) = colony_index(state, *colony_id) else {
                    return false;
                };
                #[allow(clippy::cast_possible_truncation)]
                let current = state.colonies[idx].pool.amount(commodity_id) as f32;
                #[allow(clippy::cast_possible_truncation)]
                let rate = state.colonies[idx].pool.delta(commodity_id) as f32; // per turn

                // Only fires when actually declining toward the threshold.
                if rate >= 0.0 {
                    return false;
                }
                let gap = current - threshold;
                if gap <= 0.0 {
                    // Already below threshold — fire immediately.
                    return true;
                }
                // Projected turns until crossing = gap / |rate|
                let turns_to_cross = gap / (-rate);
                turns_to_cross <= *eta_turns
            }
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn colony_index(state: &GameState, id: ColonyId) -> Option<usize> {
    state.colonies.iter().position(|c| c.id == id)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Event, GameEngine};

    /// Build an engine with one colony that has a named stockpile at a given level and delta.
    fn engine_with_stockpile(commodity: &str, amount: f64, delta: f64) -> (GameEngine, ColonyId) {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Test Colony".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;
        let idx = engine
            .state
            .colonies
            .iter()
            .position(|c| c.id == colony_id)
            .unwrap();
        engine.state.colonies[idx].pool.deposit(commodity, amount);
        // Manually poke the delta (reset_deltas zeroes it, so inject directly).
        engine.state.colonies[idx]
            .pool
            .set_delta_for_test(commodity, delta);
        (engine, colony_id)
    }

    // ── Comparison predicates ──────────────────────────────────────────────────

    #[test]
    fn less_than_fires_when_below_threshold() {
        let (engine, colony_id) = engine_with_stockpile("food", 10.0, 0.0);
        let pred = Predicate::LessThan {
            quantity: Quantity::Stockpile {
                colony_id,
                commodity_id: "food".into(),
            },
            threshold: 20.0,
        };
        assert!(PredicateEvaluator::eval(&pred, &engine.state));
    }

    #[test]
    fn less_than_does_not_fire_when_above() {
        let (engine, colony_id) = engine_with_stockpile("food", 30.0, 0.0);
        let pred = Predicate::LessThan {
            quantity: Quantity::Stockpile {
                colony_id,
                commodity_id: "food".into(),
            },
            threshold: 20.0,
        };
        assert!(!PredicateEvaluator::eval(&pred, &engine.state));
    }

    #[test]
    fn greater_than_fires_when_above_threshold() {
        let (engine, colony_id) = engine_with_stockpile("water", 50.0, 0.0);
        let pred = Predicate::GreaterThan {
            quantity: Quantity::Stockpile {
                colony_id,
                commodity_id: "water".into(),
            },
            threshold: 30.0,
        };
        assert!(PredicateEvaluator::eval(&pred, &engine.state));
    }

    #[test]
    fn stability_less_than_predicate() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Unstable".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;
        let idx = engine
            .state
            .colonies
            .iter()
            .position(|c| c.id == colony_id)
            .unwrap();
        // Force stability low.
        engine.state.populations[idx].stability = 0.15;

        let pred = Predicate::LessThan {
            quantity: Quantity::ColonyStability { colony_id },
            threshold: 0.20,
        };
        assert!(PredicateEvaluator::eval(&pred, &engine.state));
    }

    // ── Boolean composition ────────────────────────────────────────────────────

    #[test]
    fn and_requires_both_true() {
        let (engine, colony_id) = engine_with_stockpile("food", 10.0, 0.0);
        let food_low = Predicate::LessThan {
            quantity: Quantity::Stockpile {
                colony_id,
                commodity_id: "food".into(),
            },
            threshold: 20.0,
        };
        let water_low = Predicate::LessThan {
            quantity: Quantity::Stockpile {
                colony_id,
                commodity_id: "water".into(), // not seeded → 0.0 < 20.0 ✓
            },
            threshold: 20.0,
        };
        let both = Predicate::And {
            left: Box::new(food_low.clone()),
            right: Box::new(water_low.clone()),
        };
        assert!(PredicateEvaluator::eval(&both, &engine.state));

        // Now add water so the water_low predicate flips.
        let mut engine2 = GameEngine::new();
        let evs = engine2
            .apply(&Command::FoundColony {
                name: "C2".into(),
                starting_population: 10,
            })
            .unwrap();
        let Event::ColonyFounded {
            colony_id: cid2, ..
        } = &evs[0]
        else {
            panic!()
        };
        let cid2 = *cid2;
        let idx = engine2
            .state
            .colonies
            .iter()
            .position(|c| c.id == cid2)
            .unwrap();
        engine2.state.colonies[idx].pool.deposit("food", 10.0);
        engine2.state.colonies[idx].pool.deposit("water", 50.0);

        let food_low2 = Predicate::LessThan {
            quantity: Quantity::Stockpile {
                colony_id: cid2,
                commodity_id: "food".into(),
            },
            threshold: 20.0,
        };
        let water_low2 = Predicate::LessThan {
            quantity: Quantity::Stockpile {
                colony_id: cid2,
                commodity_id: "water".into(),
            },
            threshold: 20.0,
        };
        let and2 = Predicate::And {
            left: Box::new(food_low2),
            right: Box::new(water_low2),
        };
        assert!(!PredicateEvaluator::eval(&and2, &engine2.state));
    }

    #[test]
    fn or_requires_at_least_one_true() {
        let (engine, colony_id) = engine_with_stockpile("food", 5.0, 0.0);
        let food_low = Predicate::LessThan {
            quantity: Quantity::Stockpile {
                colony_id,
                commodity_id: "food".into(),
            },
            threshold: 20.0,
        };
        let water_high = Predicate::GreaterThan {
            quantity: Quantity::Stockpile {
                colony_id,
                commodity_id: "water".into(),
            },
            threshold: 100.0, // water=0 → false
        };
        let or_pred = Predicate::Or {
            left: Box::new(food_low),
            right: Box::new(water_high),
        };
        assert!(PredicateEvaluator::eval(&or_pred, &engine.state));
    }

    #[test]
    fn not_inverts_predicate() {
        let (engine, colony_id) = engine_with_stockpile("food", 30.0, 0.0);
        let food_low = Predicate::LessThan {
            quantity: Quantity::Stockpile {
                colony_id,
                commodity_id: "food".into(),
            },
            threshold: 20.0,
        };
        // food=30 → food_low is false → Not(food_low) is true
        let not_low = Predicate::Not {
            inner: Box::new(food_low),
        };
        assert!(PredicateEvaluator::eval(&not_low, &engine.state));
    }

    // ── Trend / ETA predicate ─────────────────────────────────────────────────

    #[test]
    fn declining_fires_when_crash_within_eta() {
        // food=50, rate=-5/turn → 10 turns to cross 0; eta_turns=12 → fires
        let (engine, colony_id) = engine_with_stockpile("food", 50.0, -5.0);
        let pred = Predicate::Declining {
            commodity_id: "food".into(),
            colony_id,
            threshold: 0.0,
            eta_turns: 12.0,
        };
        assert!(PredicateEvaluator::eval(&pred, &engine.state));
    }

    #[test]
    fn declining_does_not_fire_when_crash_far_away() {
        // food=200, rate=-2/turn → 100 turns to zero; eta_turns=5 → no fire
        let (engine, colony_id) = engine_with_stockpile("food", 200.0, -2.0);
        let pred = Predicate::Declining {
            commodity_id: "food".into(),
            colony_id,
            threshold: 0.0,
            eta_turns: 5.0,
        };
        assert!(!PredicateEvaluator::eval(&pred, &engine.state));
    }

    #[test]
    fn declining_does_not_fire_when_rate_positive() {
        let (engine, colony_id) = engine_with_stockpile("food", 10.0, 3.0);
        let pred = Predicate::Declining {
            commodity_id: "food".into(),
            colony_id,
            threshold: 0.0,
            eta_turns: 100.0,
        };
        assert!(!PredicateEvaluator::eval(&pred, &engine.state));
    }

    #[test]
    fn declining_fires_when_already_below_threshold() {
        // Current amount already below threshold — should fire immediately.
        let (engine, colony_id) = engine_with_stockpile("food", 0.0, -1.0);
        let pred = Predicate::Declining {
            commodity_id: "food".into(),
            colony_id,
            threshold: 10.0,
            eta_turns: 1.0,
        };
        assert!(PredicateEvaluator::eval(&pred, &engine.state));
    }

    #[test]
    fn declining_with_custom_threshold() {
        // food=100, threshold=80, rate=-5/turn → gap=20, 4 turns → eta=5 → fires
        let (engine, colony_id) = engine_with_stockpile("food", 100.0, -5.0);
        let pred = Predicate::Declining {
            commodity_id: "food".into(),
            colony_id,
            threshold: 80.0,
            eta_turns: 5.0,
        };
        assert!(PredicateEvaluator::eval(&pred, &engine.state));
    }

    // ── Missing entity → false (not panic) ────────────────────────────────────

    #[test]
    fn missing_colony_resolves_to_false() {
        let engine = GameEngine::new();
        let pred = Predicate::LessThan {
            quantity: Quantity::ColonyStability {
                colony_id: uuid::Uuid::new_v4(),
            },
            threshold: 0.5,
        };
        assert!(!PredicateEvaluator::eval(&pred, &engine.state));
    }

    // ── YAML round-trip ───────────────────────────────────────────────────────

    #[test]
    fn predicate_yaml_round_trip() {
        let id = uuid::Uuid::new_v4();
        let pred = Predicate::And {
            left: Box::new(Predicate::LessThan {
                quantity: Quantity::ColonyStability { colony_id: id },
                threshold: 0.20,
            }),
            right: Box::new(Predicate::Declining {
                commodity_id: "food".into(),
                colony_id: id,
                threshold: 0.0,
                eta_turns: 5.0,
            }),
        };
        let yaml = serde_yaml::to_string(&pred).expect("serialize");
        let back: Predicate = serde_yaml::from_str(&yaml).expect("deserialize");
        assert_eq!(pred, back);
    }

    #[test]
    fn predicate_json_round_trip() {
        let id = uuid::Uuid::new_v4();
        let pred = Predicate::Or {
            left: Box::new(Predicate::Not {
                inner: Box::new(Predicate::GreaterThan {
                    quantity: Quantity::Stockpile {
                        colony_id: id,
                        commodity_id: "water".into(),
                    },
                    threshold: 50.0,
                }),
            }),
            right: Box::new(Predicate::Declining {
                commodity_id: "food".into(),
                colony_id: id,
                threshold: 10.0,
                eta_turns: 3.0,
            }),
        };
        let json = serde_json::to_string(&pred).expect("serialize");
        let back: Predicate = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(pred, back);
        // Re-evaluate after round-trip — no engine state needed for structural equality check
        // (evaluation would need an engine, but structural equality proves round-trip fidelity).
    }

    #[test]
    fn yaml_round_trip_evaluates_identically() {
        let (engine, colony_id) = engine_with_stockpile("food", 10.0, -2.0);
        let pred = Predicate::Declining {
            commodity_id: "food".into(),
            colony_id,
            threshold: 0.0,
            eta_turns: 8.0,
        };
        let yaml = serde_yaml::to_string(&pred).expect("serialize");
        let back: Predicate = serde_yaml::from_str(&yaml).expect("deserialize");
        assert_eq!(
            PredicateEvaluator::eval(&pred, &engine.state),
            PredicateEvaluator::eval(&back, &engine.state),
            "predicate must evaluate identically after YAML round-trip"
        );
    }
}
