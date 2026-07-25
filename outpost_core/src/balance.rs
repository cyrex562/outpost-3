//! Static flow-balance calculator for colony/network configurations.
//!
//! Computes steady-state commodity net rates from a proposed set of buildings
//! and explicit import flows.  This is a **pure function** — no I/O, no game
//! state — intended for use by the balance harness, tests, and the prototyping
//! loop described in `docs/DESIGN.md §13`.
//!
//! # Usage
//!
//! ```rust
//! use outpost_core::balance::{BalanceCalculator, BuildingInstance, Flow};
//! use outpost_core::content::types::{BuildingCategory, BuildingDef, Ingredient, RecipeDef};
//!
//! let mine_def = BuildingDef {
//!     id: "mine".into(),
//!     name: "Mine".into(),
//!     description: String::new(),
//!     category: BuildingCategory::Production,
//!     construction_cost: vec![],
//!     power_delta: 0.0,
//!     worker_slots: 2,
//!     labor_required: 1,
//!     slot_cost: 1,
//!     construction_turns: 1,
//!     tech_prerequisite: None,
//!     maintenance: vec![],
//! };
//! let mine_recipe = RecipeDef {
//!     id: "mine_ore".into(),
//!     name: "Mine Ore".into(),
//!     building: "mine".into(),
//!     inputs: vec![],
//!     outputs: vec![Ingredient { id: "iron_ore".into(), quantity: 10.0 }],
//!     cycle_sols: 1,
//!     power_draw: 0.0,
//!     concurrent: false,
//! };
//! let buildings = vec![BuildingInstance {
//!     building: mine_def,
//!     recipe: Some(mine_recipe),
//!     concurrent_recipes: vec![],
//! }];
//! let report = BalanceCalculator::compute(&buildings, &[]);
//! ```

use std::collections::HashMap;

use crate::content::types::{BuildingDef, RecipeDef};

// ─── Threshold constants ──────────────────────────────────────────────────────

/// Minimum net production rate (units/sol) above which a commodity is
/// considered "comfortably surplus" for the [`BalanceVerdict::Trivial`] verdict.
///
/// If every consumed commodity shows a net exceeding this threshold, the
/// configuration is classed as Trivial (no binding constraint).
const TRIVIAL_SURPLUS_THRESHOLD: f64 = 1.0;

// ─── Public input types ───────────────────────────────────────────────────────

/// An explicit external import flow (commodity + rate in units/sol).
#[derive(Debug, Clone)]
pub struct Flow {
    /// Commodity identifier (matches [`CommodityDef::id`]).
    pub commodity_id: String,
    /// Rate in units per colony-sol (must be positive for an import).
    pub rate: f64,
}

/// A placed building instance paired with its active recipe (if any).
///
/// Buildings without a recipe (e.g. housing, storage) contribute no flows
/// and are silently ignored by the calculator.
#[derive(Debug, Clone)]
pub struct BuildingInstance {
    /// The building definition (category, power delta, etc.).
    pub building: BuildingDef,
    /// The recipe this building is running, or `None` if it has no recipe.
    pub recipe: Option<RecipeDef>,
    /// Always-on ([`RecipeDef::concurrent`]) recipes this building runs
    /// *alongside* [`Self::recipe`], every turn (issue #272).
    ///
    /// Lets one instance represent a real multi-function building. Check bundles
    /// previously faked `colony_hq` as three co-located instances all naming the
    /// same `building_id`; that gave the correct answer — [`BalanceCalculator`]
    /// reads nothing but recipe flows, so the per-building fields were never
    /// double-counted — but it described a colony that doesn't exist.
    ///
    /// **This does not model shared-scale throttling**, and cannot: the
    /// calculator is a steady-state net-rate accumulator with no scale factor for
    /// *any* building. Shortfall behaviour is the engine's job (see
    /// `colony::production`), which is why the engine has its own tests for it.
    pub concurrent_recipes: Vec<RecipeDef>,
}

// ─── Output types ─────────────────────────────────────────────────────────────

/// Steady-state balance for one commodity across the full configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommodityBalance {
    /// Commodity identifier.
    pub commodity_id: String,
    /// Total production rate (buildings + imports) in units/sol.
    pub production_rate: f64,
    /// Total consumption rate in units/sol.
    pub consumption_rate: f64,
    /// `production_rate − consumption_rate`; negative means deficit.
    pub net_rate: f64,
}

/// Overall classification verdict for a colony/network configuration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BalanceVerdict {
    /// All commodity nets ≥ 0 and at least one is a binding constraint.
    Closed,
    /// The chain is unsustainable: commodity is in deficit (`net_rate < 0`).
    ///
    /// Contains the commodity id and the deficit rate (negative value).
    Bottleneck(String, f64),
    /// A required commodity has **no source at all** (zero production and
    /// zero imports), making the chain structurally impossible to run.
    Impossible(String),
    /// All nets are significantly positive; the configuration has no binding
    /// constraint and is likely uninteresting for balance tuning.
    Trivial,
}

/// Full output of a [`BalanceCalculator::compute`] call.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BalanceReport {
    /// Per-commodity steady-state balances, sorted by commodity id.
    pub commodities: Vec<CommodityBalance>,
    /// Overall classification of the configuration.
    pub verdict: BalanceVerdict,
}

impl BalanceReport {
    /// Look up the balance entry for a specific commodity id, if present.
    #[must_use]
    pub fn commodity(&self, id: &str) -> Option<&CommodityBalance> {
        self.commodities.iter().find(|c| c.commodity_id == id)
    }
}

// ─── Calculator ───────────────────────────────────────────────────────────────

/// Steady-state flow-balance calculator.
///
/// All methods are pure functions — no I/O, no mutation of game state.
pub struct BalanceCalculator;

impl BalanceCalculator {
    /// Compute the steady-state balance for a proposed configuration.
    ///
    /// # Arguments
    ///
    /// * `buildings` — placed building instances with their active recipes
    /// * `imports`   — explicit external flows added on top of building production
    ///
    /// # Returns
    ///
    /// A [`BalanceReport`] with per-commodity net rates and an overall verdict.
    #[must_use]
    pub fn compute(buildings: &[BuildingInstance], imports: &[Flow]) -> BalanceReport {
        // Accumulate production and consumption rates per commodity.
        // We track them separately so we can detect "no source" (Impossible).
        let mut production: HashMap<String, f64> = HashMap::new();
        let mut consumption: HashMap<String, f64> = HashMap::new();

        // ── Imports ──────────────────────────────────────────────────────────
        for flow in imports {
            if flow.rate > 0.0 {
                *production.entry(flow.commodity_id.clone()).or_default() += flow.rate;
            }
        }

        // ── Building recipes (rate = quantity / cycle_sols) ───────────────────
        for inst in buildings {
            // The pick-one recipe plus every always-on one (issue #272) — the
            // same set the engine's production step runs together.
            for recipe in inst.recipe.iter().chain(&inst.concurrent_recipes) {
                let cycle = f64::from(recipe.cycle_sols.max(1));
                for input in &recipe.inputs {
                    if input.quantity > 0.0 {
                        *consumption.entry(input.id.clone()).or_default() += input.quantity / cycle;
                    }
                }
                for output in &recipe.outputs {
                    if output.quantity > 0.0 {
                        *production.entry(output.id.clone()).or_default() +=
                            output.quantity / cycle;
                    }
                }
            }
        }

        // ── Build per-commodity balance rows ─────────────────────────────────
        let mut all_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        all_ids.extend(production.keys().cloned());
        all_ids.extend(consumption.keys().cloned());

        let mut commodities: Vec<CommodityBalance> = all_ids
            .into_iter()
            .map(|id| {
                let prod = production.get(&id).copied().unwrap_or(0.0);
                let cons = consumption.get(&id).copied().unwrap_or(0.0);
                CommodityBalance {
                    commodity_id: id,
                    production_rate: prod,
                    consumption_rate: cons,
                    net_rate: prod - cons,
                }
            })
            .collect();

        // Sort deterministically by commodity id (already BTreeSet order).
        commodities.sort_by(|a, b| a.commodity_id.cmp(&b.commodity_id));

        // ── Classify ─────────────────────────────────────────────────────────
        let verdict = Self::classify(&commodities);

        BalanceReport {
            commodities,
            verdict,
        }
    }

    /// Classify a set of commodity balances into a [`BalanceVerdict`].
    fn classify(commodities: &[CommodityBalance]) -> BalanceVerdict {
        // Pass 1: Impossible — a commodity that is consumed but never produced.
        for cb in commodities {
            if cb.consumption_rate > 0.0 && cb.production_rate <= 0.0 {
                return BalanceVerdict::Impossible(cb.commodity_id.clone());
            }
        }

        // Pass 2: Bottleneck — find the worst deficit (most negative net).
        let worst_deficit = commodities
            .iter()
            .filter(|cb| cb.net_rate < 0.0)
            .min_by(|a, b| a.net_rate.partial_cmp(&b.net_rate).unwrap());

        if let Some(cb) = worst_deficit {
            return BalanceVerdict::Bottleneck(cb.commodity_id.clone(), cb.net_rate);
        }

        // Pass 3: Trivial — every consumed commodity has a comfortable surplus.
        let all_trivial = commodities
            .iter()
            .filter(|cb| cb.consumption_rate > 0.0)
            .all(|cb| cb.net_rate >= TRIVIAL_SURPLUS_THRESHOLD);

        // Also trivial if there are no consumed commodities but production exists
        // (pure extractors with no downstream — nothing constraining).
        let has_any_consumption = commodities.iter().any(|cb| cb.consumption_rate > 0.0);
        let has_any_production = commodities.iter().any(|cb| cb.production_rate > 0.0);

        if (all_trivial && has_any_consumption) || (!has_any_consumption && has_any_production) {
            return BalanceVerdict::Trivial;
        }

        // Pass 4: Closed — all nets ≥ 0, at least one binding.
        BalanceVerdict::Closed
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{BuildingCategory, BuildingDef, Ingredient, RecipeDef};

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn make_building(id: &str) -> BuildingDef {
        BuildingDef {
            id: id.into(),
            name: id.into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        }
    }

    fn make_recipe(
        id: &str,
        building: &str,
        inputs: Vec<(&str, f64)>,
        outputs: Vec<(&str, f64)>,
    ) -> RecipeDef {
        RecipeDef {
            id: id.into(),
            name: id.into(),
            building: building.into(),
            inputs: inputs
                .into_iter()
                .map(|(cid, qty)| Ingredient {
                    id: cid.into(),
                    quantity: qty,
                })
                .collect(),
            outputs: outputs
                .into_iter()
                .map(|(cid, qty)| Ingredient {
                    id: cid.into(),
                    quantity: qty,
                })
                .collect(),
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: false,
        }
    }

    fn instance(building_id: &str, recipe: Option<RecipeDef>) -> BuildingInstance {
        BuildingInstance {
            building: make_building(building_id),
            recipe,
            concurrent_recipes: vec![],
        }
    }

    /// A multi-function building: one instance running several always-on
    /// recipes at once (issue #272).
    fn multi_function_instance(
        building_id: &str,
        recipe: Option<RecipeDef>,
        concurrent_recipes: Vec<RecipeDef>,
    ) -> BuildingInstance {
        BuildingInstance {
            building: make_building(building_id),
            recipe,
            concurrent_recipes,
        }
    }

    // ── Multi-function buildings (issue #272) ────────────────────────────

    /// One instance running several always-on recipes must account for all of
    /// them — that is the whole point of the field.
    #[test]
    fn a_multi_function_instance_counts_every_concurrent_recipe() {
        let power = make_recipe("hq_power", "colony_hq", vec![], vec![("power", 24.0)]);
        let water = make_recipe("hq_water", "colony_hq", vec![], vec![("water", 24.0)]);
        let sink = make_recipe(
            "consume",
            "sink",
            vec![("power", 20.0), ("water", 15.0)],
            vec![],
        );

        let buildings = vec![
            multi_function_instance("colony_hq", None, vec![power, water]),
            instance("sink", Some(sink)),
        ];
        let report = BalanceCalculator::compute(&buildings, &[]);

        let net = |id: &str| {
            report
                .commodities
                .iter()
                .find(|c| c.commodity_id == id)
                .map_or(0.0, |c| c.net_rate)
        };
        assert!(
            (net("power") - 4.0).abs() < 1e-9,
            "power net was {}",
            net("power")
        );
        assert!(
            (net("water") - 9.0).abs() < 1e-9,
            "water net was {}",
            net("water")
        );
    }

    /// The equivalence the `colony_hq_efficiency` bundle relied on before #272:
    /// N co-located single-recipe instances and one multi-recipe instance give
    /// identical results, because the calculator reads nothing but recipe flows.
    ///
    /// Pinned so the claim in that bundle's `pack.yaml` stays true, and so a
    /// future change that starts reading `BuildingDef` fields (`power_delta`,
    /// `slot_cost`) has to notice it is breaking the old pattern.
    #[test]
    fn one_multi_recipe_instance_equals_n_co_located_instances() {
        let power = make_recipe("hq_power", "colony_hq", vec![], vec![("power", 24.0)]);
        let water = make_recipe("hq_water", "colony_hq", vec![], vec![("water", 24.0)]);

        let combined = vec![multi_function_instance(
            "colony_hq",
            None,
            vec![power.clone(), water.clone()],
        )];
        let faked = vec![
            instance("colony_hq", Some(power)),
            instance("colony_hq", Some(water)),
        ];

        let a = BalanceCalculator::compute(&combined, &[]);
        let b = BalanceCalculator::compute(&faked, &[]);
        assert_eq!(a.commodities.len(), b.commodities.len());
        for (x, y) in a.commodities.iter().zip(&b.commodities) {
            assert_eq!(x.commodity_id, y.commodity_id);
            assert!(
                (x.net_rate - y.net_rate).abs() < 1e-9,
                "{} diverged: {} vs {}",
                x.commodity_id,
                x.net_rate,
                y.net_rate
            );
        }
    }

    // ── Case 1: Closed ───────────────────────────────────────────────────────

    /// A mine produces 10 ore/sol, a smelter consumes 10 ore and produces 5 plates.
    /// Exactly balanced on ore → Closed.
    #[test]
    fn closed_chain_reports_closed() {
        let mine_recipe = make_recipe("mine_ore", "mine", vec![], vec![("ore", 10.0)]);
        let smelt_recipe = make_recipe(
            "smelt",
            "smelter",
            vec![("ore", 10.0)],
            vec![("plate", 5.0)],
        );

        let buildings = vec![
            instance("mine", Some(mine_recipe)),
            instance("smelter", Some(smelt_recipe)),
        ];

        let report = BalanceCalculator::compute(&buildings, &[]);

        assert_eq!(
            report.verdict,
            BalanceVerdict::Closed,
            "expected Closed, got {:?}",
            report.verdict
        );

        let ore_bal = report.commodity("ore").expect("ore should appear");
        assert!(
            ore_bal.net_rate.abs() < 1e-9,
            "ore net should be ~0, got {}",
            ore_bal.net_rate
        );

        let plate_bal = report.commodity("plate").expect("plate should appear");
        assert!(
            plate_bal.net_rate > 0.0,
            "plate should have positive net, got {}",
            plate_bal.net_rate
        );
    }

    // ── Case 2: Impossible ───────────────────────────────────────────────────

    /// A smelter requires ore but there is no ore source.
    #[test]
    fn missing_input_reports_impossible() {
        let smelt_recipe =
            make_recipe("smelt", "smelter", vec![("ore", 5.0)], vec![("plate", 2.0)]);

        let buildings = vec![instance("smelter", Some(smelt_recipe))];

        let report = BalanceCalculator::compute(&buildings, &[]);

        assert!(
            matches!(report.verdict, BalanceVerdict::Impossible(ref id) if id == "ore"),
            "expected Impossible(ore), got {:?}",
            report.verdict
        );
    }

    // ── Case 3: Trivial ──────────────────────────────────────────────────────

    /// A mine produces 100 ore/sol, a smelter consumes only 1 ore — large surplus.
    #[test]
    fn large_surplus_reports_trivial() {
        let mine_recipe = make_recipe("mine_ore", "mine", vec![], vec![("ore", 100.0)]);
        let smelt_recipe =
            make_recipe("smelt", "smelter", vec![("ore", 1.0)], vec![("plate", 1.0)]);

        let buildings = vec![
            instance("mine", Some(mine_recipe)),
            instance("smelter", Some(smelt_recipe)),
        ];

        let report = BalanceCalculator::compute(&buildings, &[]);

        assert_eq!(
            report.verdict,
            BalanceVerdict::Trivial,
            "expected Trivial, got {:?}",
            report.verdict
        );

        let ore_bal = report.commodity("ore").expect("ore should appear");
        assert!(
            ore_bal.net_rate > TRIVIAL_SURPLUS_THRESHOLD,
            "ore surplus should exceed trivial threshold, got {}",
            ore_bal.net_rate
        );
    }

    // ── Case 4: Bottleneck ───────────────────────────────────────────────────

    /// Mine produces 10 ore/sol; two smelters each consume 8 ore/sol → deficit.
    #[test]
    fn choke_point_reports_bottleneck() {
        let mine_recipe = make_recipe("mine_ore", "mine", vec![], vec![("ore", 10.0)]);
        let smelt1 = make_recipe(
            "smelt_a",
            "smelter_a",
            vec![("ore", 8.0)],
            vec![("plate", 4.0)],
        );
        let smelt2 = make_recipe(
            "smelt_b",
            "smelter_b",
            vec![("ore", 8.0)],
            vec![("plate", 4.0)],
        );

        let buildings = vec![
            instance("mine", Some(mine_recipe)),
            instance("smelter_a", Some(smelt1)),
            instance("smelter_b", Some(smelt2)),
        ];

        let report = BalanceCalculator::compute(&buildings, &[]);

        assert!(
            matches!(report.verdict, BalanceVerdict::Bottleneck(ref id, deficit) if id == "ore" && deficit < 0.0),
            "expected Bottleneck(ore, <0), got {:?}",
            report.verdict
        );

        let ore_bal = report.commodity("ore").expect("ore should appear");
        // net = 10 - 16 = -6
        assert!(
            (ore_bal.net_rate - (-6.0)).abs() < 1e-9,
            "ore net should be -6, got {}",
            ore_bal.net_rate
        );
    }

    // ── Import flows satisfy missing input ───────────────────────────────────

    /// Same smelter as in the Impossible test, but with an import flow for ore.
    /// The import makes it sustainable → Closed (ore net = import - consumption).
    #[test]
    fn import_satisfies_missing_input() {
        let smelt_recipe =
            make_recipe("smelt", "smelter", vec![("ore", 5.0)], vec![("plate", 2.0)]);

        let buildings = vec![instance("smelter", Some(smelt_recipe))];
        let imports = vec![Flow {
            commodity_id: "ore".into(),
            rate: 5.0,
        }];

        let report = BalanceCalculator::compute(&buildings, &imports);

        assert_eq!(
            report.verdict,
            BalanceVerdict::Closed,
            "import should resolve Impossible to Closed; got {:?}",
            report.verdict
        );

        let ore_bal = report.commodity("ore").expect("ore should appear");
        assert!(
            ore_bal.net_rate.abs() < 1e-9,
            "ore net should be 0 with import == consumption, got {}",
            ore_bal.net_rate
        );
    }

    // ── Multi-cycle recipes ──────────────────────────────────────────────────

    /// A recipe with cycle_sols = 2 should halve the effective flow rate.
    #[test]
    fn multi_cycle_recipe_halves_effective_rate() {
        let mut recipe = make_recipe("slow_mine", "mine", vec![], vec![("ore", 10.0)]);
        recipe.cycle_sols = 2;

        let buildings = vec![instance("mine", Some(recipe))];
        let report = BalanceCalculator::compute(&buildings, &[]);

        let ore = report.commodity("ore").expect("ore should appear");
        assert!(
            (ore.production_rate - 5.0).abs() < 1e-9,
            "rate should be 10/2=5, got {}",
            ore.production_rate
        );
    }
}
