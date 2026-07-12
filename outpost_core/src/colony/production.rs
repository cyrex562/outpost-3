//! Per-turn production resolution for colony buildings.
//!
//! Each turn, operational buildings scale their output by
//! `min(input_ratio, power_ratio, labor_ratio)` — a continuous scalar, not a
//! binary on/off step. Shortfalls cause partial production, never a panic.
//!
//! See `docs/DESIGN.md §5, §7`. The C#
//! `ColonyTurnProcessor.ProcessProduction()` is the behavioural spec;
//! the key improvement here is that scaling is continuous.

use crate::content::types::Ingredient;
use crate::content::{BuildingCategory, ContentRegistry, RecipeDef};

use super::pool::ColonyPool;

// ─── Power Grid ──────────────────────────────────────────────────────────────

/// Colony power grid state computed at the start of each production step.
#[derive(Debug, Clone)]
pub struct PowerGrid {
    /// Total power capacity available (kW) from all power-generating buildings.
    pub capacity: f64,
    /// Total power demanded (kW) by all operational buildings and recipes.
    pub demand: f64,
}

impl PowerGrid {
    /// Fraction of demanded power that is available, in `[0.0, 1.0]`.
    ///
    /// Returns `1.0` when supply meets or exceeds demand (including the zero
    /// demand case). Returns a value in `(0.0, 1.0)` during a brownout.
    #[must_use]
    pub fn supply_ratio(&self) -> f64 {
        if self.demand <= 0.0 {
            return 1.0;
        }
        (self.capacity / self.demand).min(1.0)
    }
}

// ─── Production results ───────────────────────────────────────────────────────

/// A shortfall that limited production for one building this turn.
#[derive(Debug, Clone)]
pub struct ProductionShortfall {
    /// Human-readable description of what was short.
    pub reason: ShortfallReason,
    /// The scale factor that was actually applied (`< 1.0` when short).
    pub effective_scale: f64,
}

/// Category of shortfall limiting a building's production.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ShortfallReason {
    /// One or more input commodities were insufficient.
    InputShort {
        /// The commodity id that was the tightest constraint.
        commodity_id: String,
    },
    /// Power grid brownout reduced output.
    PowerBrownout,
    /// Insufficient colony labour.
    LaborShort,
    /// Per-sol maintenance draw could not be satisfied (issue #180).
    ///
    /// Emitted when the tightest constraint on the building's effective input
    /// ratio is a commodity that appears **only** in
    /// [`crate::content::BuildingDef::maintenance`], not in the recipe inputs.
    /// A shared commodity (input + maintenance) still reports as
    /// [`ShortfallReason::InputShort`].
    MaintenanceShort {
        /// The maintenance commodity id that was the tightest constraint.
        commodity_id: String,
    },
}

/// Outcome of one building's production attempt this turn.
#[derive(Debug, Clone)]
pub struct BuildingProductionResult {
    /// Content-pack key of the building type.
    pub building_type: String,
    /// Recipe that ran (or was attempted).
    pub recipe_id: String,
    /// Scale factor applied to all inputs and outputs, in `[0.0, 1.0]`.
    pub scale: f64,
    /// Shortfalls that reduced the scale below 1.0, if any.
    pub shortfalls: Vec<ProductionShortfall>,
}

impl BuildingProductionResult {
    /// `true` if the building ran at full capacity.
    #[must_use]
    pub fn is_full_production(&self) -> bool {
        self.shortfalls.is_empty() && (self.scale - 1.0).abs() < 1e-9
    }
}

/// Summary output of one colony's production step.
#[derive(Debug, Clone)]
pub struct ProductionStepOutcome {
    /// Per-building results for every building that attempted production.
    pub building_results: Vec<BuildingProductionResult>,
    /// Power grid state computed at the start of the step.
    pub power_grid: PowerGrid,
    /// Labor available vs labor demanded this turn (fractional).
    pub labor_available: f32,
    /// Total labor demanded by all operational production buildings.
    pub labor_demanded: f32,
}

// ─── Internal helpers for two-pass production resolution ─────────────────────

/// Holds a building's pre-computed scale before any pool mutations occur.
struct PendingProduction<'a> {
    building_type: &'a str,
    recipe: Option<&'a RecipeDef>,
    maintenance: &'a [Ingredient],
    /// Effective per-sol maintenance multiplier
    /// (`MaintenanceConsumption` scalar; `0.0` when disabled).
    maintenance_multiplier: f64,
    scale: f64,
    shortfalls: Vec<ProductionShortfall>,
}

// ─── Production resolution ────────────────────────────────────────────────────

/// Run the production step for a single colony.
///
/// For each operational building that has an associated recipe, this function:
/// 1. Computes a continuous scale factor `min(input_ratio, power_ratio, labor_ratio)`.
/// 2. Withdraws `scale × input_qty` from the pool for each input.
/// 3. Deposits `scale × output_qty` into the pool for each output.
/// 4. Records any shortfalls in the returned [`ProductionStepOutcome`].
///
/// Buildings with `category == Power` are essential and never browned out;
/// all other buildings receive the grid's `supply_ratio` as their `power_ratio`.
///
/// # Arguments
///
/// * `pool`            — mutable colony stockpile (modified in-place)
/// * `buildings`       — slice of `(building_type, slot_cost)` pairs for placed buildings
/// * `labor_available` — labour units available this turn (from `PopulationPool::available_labor`)
/// * `registry`        — content registry for looking up `BuildingDef` and `RecipeDef`
pub fn process_production(
    pool: &mut ColonyPool,
    buildings: &[(String, u32)],
    labor_available: f32,
    registry: &ContentRegistry,
) -> ProductionStepOutcome {
    process_production_scaled(pool, buildings, labor_available, registry, 1.0, 1.0, true)
}

/// Same as [`process_production`] but multiplies positive `power_delta` and
/// `recipe.power_draw` (consumers only) by `power_scalar` (the resolved
/// `PowerRequirement` difficulty scalar, #161) and per-sol
/// [`crate::content::BuildingDef::maintenance`] draws by `maintenance_scalar`
/// (the resolved `MaintenanceConsumption` difficulty scalar, #180).
///
/// Generators (negative `power_delta`) are unaffected by `power_scalar`.
/// When `maintenance_enabled` is `false` the maintenance draw is short-
/// circuited regardless of `maintenance_scalar` (the master toggle).
pub fn process_production_scaled(
    pool: &mut ColonyPool,
    buildings: &[(String, u32)],
    labor_available: f32,
    registry: &ContentRegistry,
    power_scalar: f32,
    maintenance_scalar: f32,
    maintenance_enabled: bool,
) -> ProductionStepOutcome {
    // ── Step 1: build power grid ─────────────────────────────────────────────
    let power_grid = compute_power_grid_scaled(buildings, registry, power_scalar);
    let brownout_ratio = power_grid.supply_ratio();

    // ── Step 2: compute labor ratio ──────────────────────────────────────────
    let labor_demanded: f32 = buildings
        .iter()
        .filter_map(|(bt, _)| registry.building(bt))
        .filter(|bd| has_recipe(bd.id.as_str(), registry))
        .map(|bd| {
            #[allow(clippy::cast_precision_loss)]
            {
                bd.worker_slots as f32
            }
        })
        .sum();

    let labor_ratio = if labor_demanded <= 0.0 {
        1.0_f64
    } else {
        (f64::from(labor_available) / f64::from(labor_demanded)).min(1.0)
    };

    // ── Step 3: two-pass resolution ───────────────────────────────────────────
    //
    // Pass A: compute scales based on the *start-of-turn* pool state so that
    //         mines and other producers don't inflate inputs for downstream
    //         buildings in the same turn.
    // Pass B: apply all scaled changes to the pool.
    //
    // This avoids order-dependency and matches the C# "snapshot then apply"
    // behaviour described in the behavioural spec.

    let mut pending: Vec<PendingProduction<'_>> = Vec::new();
    let maintenance_multiplier = if maintenance_enabled {
        f64::from(maintenance_scalar.max(0.0))
    } else {
        0.0
    };

    for (building_type, _slot_cost) in buildings {
        let Some(bdef) = registry.building(building_type) else {
            continue; // unknown building type — skip
        };
        let recipe = first_recipe_for_building(building_type, registry);
        let has_maintenance = maintenance_enabled && !bdef.maintenance.is_empty();

        // Buildings with neither a recipe nor an active maintenance list stay
        // out of the production pass entirely (unchanged pre-#180 behaviour).
        if recipe.is_none() && !has_maintenance {
            continue;
        }

        // Combined per-commodity demand (recipe inputs + maintenance).
        let (input_ratio, tight_commodity, tight_is_maintenance) =
            compute_effective_input_ratio(
                pool,
                recipe,
                if has_maintenance { bdef.maintenance.as_slice() } else { &[] },
                maintenance_multiplier,
            );

        // Power/labor ratios only apply to recipe-running buildings. A pure
        // maintenance-only building doesn't consume labour or drive brownouts.
        let (power_ratio, applies_labor) = if recipe.is_some() {
            let pr = if bdef.category == BuildingCategory::Power {
                1.0
            } else {
                brownout_ratio
            };
            (pr, true)
        } else {
            (1.0, false)
        };
        let effective_labor_ratio = if applies_labor { labor_ratio } else { 1.0 };

        // Overall scale factor.
        let scale = input_ratio
            .min(power_ratio)
            .min(effective_labor_ratio)
            .max(0.0);

        // Record shortfalls. Maintenance-only tight constraints report as
        // `MaintenanceShort`; shared input+maintenance constraints stay as
        // `InputShort` for backwards compatibility with pre-#180 events.
        let mut shortfalls: Vec<ProductionShortfall> = Vec::new();

        if input_ratio < 1.0 - 1e-9 {
            let commodity_id = tight_commodity.unwrap_or_default();
            let reason = if tight_is_maintenance {
                ShortfallReason::MaintenanceShort { commodity_id }
            } else {
                ShortfallReason::InputShort { commodity_id }
            };
            shortfalls.push(ProductionShortfall {
                reason,
                effective_scale: input_ratio,
            });
        }
        if power_ratio < 1.0 - 1e-9 {
            shortfalls.push(ProductionShortfall {
                reason: ShortfallReason::PowerBrownout,
                effective_scale: power_ratio,
            });
        }
        if effective_labor_ratio < 1.0 - 1e-9 {
            shortfalls.push(ProductionShortfall {
                reason: ShortfallReason::LaborShort,
                effective_scale: effective_labor_ratio,
            });
        }

        pending.push(PendingProduction {
            building_type,
            recipe,
            maintenance: if has_maintenance {
                bdef.maintenance.as_slice()
            } else {
                &[]
            },
            maintenance_multiplier,
            scale,
            shortfalls,
        });
    }

    // Pass B: apply all changes now that every scale has been determined.
    let mut building_results: Vec<BuildingProductionResult> = Vec::new();
    for p in pending {
        if p.scale > 1e-9 {
            if let Some(recipe) = p.recipe {
                for ingredient in &recipe.inputs {
                    pool.withdraw(&ingredient.id, ingredient.quantity * p.scale);
                }
                for ingredient in &recipe.outputs {
                    pool.deposit(&ingredient.id, ingredient.quantity * p.scale);
                }
            }
            for ingredient in p.maintenance {
                pool.withdraw(
                    &ingredient.id,
                    ingredient.quantity * p.maintenance_multiplier * p.scale,
                );
            }
        }
        let recipe_id = p
            .recipe
            .map(|r| r.id.clone())
            .unwrap_or_default();
        building_results.push(BuildingProductionResult {
            building_type: p.building_type.to_owned(),
            recipe_id,
            scale: p.scale,
            shortfalls: p.shortfalls,
        });
    }

    ProductionStepOutcome {
        building_results,
        power_grid,
        labor_available,
        labor_demanded,
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Compute the colony power grid, scaling consumer power draws by
/// `power_scalar` (issue #161). Generators are unaffected. Pass `1.0` for
/// the neutral (no-difficulty) case.
fn compute_power_grid_scaled(
    buildings: &[(String, u32)],
    registry: &ContentRegistry,
    power_scalar: f32,
) -> PowerGrid {
    let mut capacity = 0.0f64;
    let mut demand = 0.0f64;
    let mul = f64::from(power_scalar.max(0.0));

    for (building_type, _) in buildings {
        let Some(bdef) = registry.building(building_type) else {
            continue;
        };
        // Negative power_delta = generator.
        if bdef.power_delta < 0.0 {
            capacity += -bdef.power_delta;
        } else {
            demand += bdef.power_delta * mul;
        }
        // Recipe power draw adds to demand.
        if let Some(recipe) = first_recipe_for_building(building_type, registry) {
            demand += recipe.power_draw * mul;
        }
    }

    PowerGrid { capacity, demand }
}

/// Compute the effective input ratio for a building, combining recipe inputs
/// with scaled maintenance draws (issue #180).
///
/// Returns `(ratio, tight_commodity_id, tight_is_maintenance_only)`.
///
/// Per-commodity demand is summed across recipe inputs and maintenance entries
/// so the pool is never double-counted. The returned `tight_is_maintenance_only`
/// is `true` only when the tightest constraint is a commodity that appears
/// exclusively in the maintenance list — this drives the `MaintenanceShort`
/// vs. `InputShort` attribution.
fn compute_effective_input_ratio(
    pool: &ColonyPool,
    recipe: Option<&RecipeDef>,
    maintenance: &[Ingredient],
    maintenance_multiplier: f64,
) -> (f64, Option<String>, bool) {
    // Merged (id, quantity, has_recipe_demand) list, preserving deterministic
    // order: recipe inputs first, then maintenance entries (with dedup).
    let mut demands: Vec<(String, f64, bool)> = Vec::new();

    if let Some(r) = recipe {
        for ing in &r.inputs {
            if ing.quantity > 0.0 {
                demands.push((ing.id.clone(), ing.quantity, true));
            }
        }
    }
    if maintenance_multiplier > 0.0 {
        for ing in maintenance {
            if ing.quantity <= 0.0 {
                continue;
            }
            let scaled = ing.quantity * maintenance_multiplier;
            if let Some(existing) = demands.iter_mut().find(|d| d.0 == ing.id) {
                existing.1 += scaled;
            } else {
                demands.push((ing.id.clone(), scaled, false));
            }
        }
    }

    let mut ratio = 1.0f64;
    let mut tight: Option<String> = None;
    let mut tight_is_maintenance = false;

    for (id, qty, has_recipe_demand) in &demands {
        let available = pool.amount(id);
        let r = (available / *qty).min(1.0);
        if r < ratio {
            ratio = r;
            tight = Some(id.clone());
            tight_is_maintenance = !*has_recipe_demand;
        }
    }

    (ratio.max(0.0), tight, tight_is_maintenance)
}

/// Return the first recipe whose `building` field matches `building_type`.
fn first_recipe_for_building<'r>(
    building_type: &str,
    registry: &'r ContentRegistry,
) -> Option<&'r RecipeDef> {
    registry.recipes().find(|r| r.building == building_type)
}

/// Returns true if there is at least one recipe for the given building type.
fn has_recipe(building_type: &str, registry: &ContentRegistry) -> bool {
    first_recipe_for_building(building_type, registry).is_some()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{BuildingCategory, BuildingDef, Ingredient, RecipeDef};
    use crate::content::ContentRegistry;

    // ── Registry builders ────────────────────────────────────────────────────

    fn make_registry_with_power() -> ContentRegistry {
        let mut reg = ContentRegistry::default();

        // Power plant: produces 100 kW
        reg.insert_building(BuildingDef {
            id: "solar_array".into(),
            name: "Solar Array".into(),
            description: String::new(),
            category: BuildingCategory::Power,
            construction_cost: vec![],
            power_delta: -100.0, // negative = producer
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });

        // Mine: extracts ore; needs 30 kW; 2 workers
        reg.insert_building(BuildingDef {
            id: "mine".into(),
            name: "Mine".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 2,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });
        reg.insert_recipe(RecipeDef {
            id: "mine_ore".into(),
            name: "Mine Ore".into(),
            building: "mine".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "iron_ore".into(),
                quantity: 10.0,
            }],
            cycle_sols: 1,
            power_draw: 30.0,
        });

        // Smelter: converts ore to plates; needs 50 kW; 3 workers
        reg.insert_building(BuildingDef {
            id: "smelter".into(),
            name: "Smelter".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 3,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });
        reg.insert_recipe(RecipeDef {
            id: "smelt_iron".into(),
            name: "Smelt Iron".into(),
            building: "smelter".into(),
            inputs: vec![Ingredient {
                id: "iron_ore".into(),
                quantity: 2.0,
            }],
            outputs: vec![Ingredient {
                id: "iron_plate".into(),
                quantity: 1.0,
            }],
            cycle_sols: 1,
            power_draw: 50.0,
        });

        reg
    }

    // ── Helper to list placed buildings as (type, slot_cost) ─────────────────

    fn buildings(types: &[&str]) -> Vec<(String, u32)> {
        types.iter().map(|&t| (t.to_owned(), 1u32)).collect()
    }

    // ── Full production when well-fed ────────────────────────────────────────

    #[test]
    fn full_production_when_inputs_power_labor_ample() {
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 100.0); // plenty of ore

        let placed = buildings(&["solar_array", "mine", "smelter"]);
        // labor: mine needs 2, smelter needs 3 → 5 total; give 10
        let outcome = process_production(&mut pool, &placed, 10.0_f32, &reg);

        // Mine ran at scale 1.0 (no inputs required, power ample)
        let mine_res = outcome
            .building_results
            .iter()
            .find(|r| r.building_type == "mine")
            .unwrap();
        assert!(
            mine_res.is_full_production(),
            "mine shortfalls: {:?}",
            mine_res.shortfalls
        );

        // Smelter ran at scale 1.0 (100 ore available, needs 2)
        let smelt_res = outcome
            .building_results
            .iter()
            .find(|r| r.building_type == "smelter")
            .unwrap();
        assert!(
            smelt_res.is_full_production(),
            "smelter shortfalls: {:?}",
            smelt_res.shortfalls
        );

        // After the mine runs: +10 ore. After smelter: -2 ore, +1 plate.
        // Starting ore: 100, +10 (mine), -2 (smelter) = 108
        assert!((pool.amount("iron_ore") - 108.0).abs() < 1e-6);
        assert!((pool.amount("iron_plate") - 1.0).abs() < 1e-6);
    }

    // ── Input-short: partial production ─────────────────────────────────────

    #[test]
    fn input_short_scales_down_output() {
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        // Only 1 unit of ore; smelter needs 2 → input_ratio = 0.5
        pool.deposit("iron_ore", 1.0);

        let placed = buildings(&["solar_array", "smelter"]);
        let outcome = process_production(&mut pool, &placed, 100.0_f32, &reg);

        let smelt = outcome
            .building_results
            .iter()
            .find(|r| r.building_type == "smelter")
            .unwrap();
        assert!(
            (smelt.scale - 0.5).abs() < 1e-6,
            "scale was {}",
            smelt.scale
        );
        assert!(
            smelt
                .shortfalls
                .iter()
                .any(|s| matches!(s.reason, ShortfallReason::InputShort { .. })),
            "expected InputShort shortfall"
        );

        // 1 ore × 0.5 = 0.5 consumed; 0.5 plates produced
        assert!((pool.amount("iron_ore")).abs() < 1e-6);
        assert!((pool.amount("iron_plate") - 0.5).abs() < 1e-6);
    }

    // ── Power brownout: proportional reduction ───────────────────────────────

    #[test]
    fn power_brownout_reduces_output_proportionally() {
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        // Mine needs 30 kW, smelter needs 50 kW → 80 kW demand.
        // Solar array provides 100 kW. Add another mine to push demand > supply.
        pool.deposit("iron_ore", 100.0);

        // Two mines (60 kW) + smelter (50 kW) = 110 kW demand, 100 kW supply
        // brownout_ratio = 100 / 110 ≈ 0.909...
        let placed = buildings(&["solar_array", "mine", "mine", "smelter"]);
        let outcome = process_production(&mut pool, &placed, 100.0_f32, &reg);

        let expected_ratio = 100.0_f64 / 110.0;
        assert!(
            (outcome.power_grid.supply_ratio() - expected_ratio).abs() < 1e-6,
            "supply_ratio was {}",
            outcome.power_grid.supply_ratio()
        );

        for res in &outcome.building_results {
            if res.building_type == "mine" || res.building_type == "smelter" {
                assert!(
                    (res.scale - expected_ratio).abs() < 1e-6,
                    "building {} scale was {} (expected ~{})",
                    res.building_type,
                    res.scale,
                    expected_ratio
                );
                assert!(
                    res.shortfalls
                        .iter()
                        .any(|s| s.reason == ShortfallReason::PowerBrownout),
                    "expected PowerBrownout shortfall for {}",
                    res.building_type
                );
            }
        }
    }

    // ── Labor-short: partial production ──────────────────────────────────────

    #[test]
    fn labor_short_scales_down_output() {
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 100.0);

        // mine needs 2 workers + smelter needs 3 workers = 5 total.
        // Give only 2 → labor_ratio = 2/5 = 0.4
        let placed = buildings(&["solar_array", "mine", "smelter"]);
        let outcome = process_production(&mut pool, &placed, 2.0_f32, &reg);

        let expected_ratio = 2.0_f64 / 5.0;

        for res in &outcome.building_results {
            if res.building_type == "mine" || res.building_type == "smelter" {
                assert!(
                    (res.scale - expected_ratio).abs() < 1e-6,
                    "building {} scale was {} (expected {})",
                    res.building_type,
                    res.scale,
                    expected_ratio
                );
                assert!(
                    res.shortfalls
                        .iter()
                        .any(|s| s.reason == ShortfallReason::LaborShort),
                    "expected LaborShort for {}",
                    res.building_type
                );
            }
        }
    }

    // ── Deterministic for fixed seed ─────────────────────────────────────────

    #[test]
    fn production_is_deterministic_given_same_inputs() {
        let reg = make_registry_with_power();

        let run = || {
            let mut pool = ColonyPool::new();
            pool.deposit("iron_ore", 5.0);
            let placed = buildings(&["solar_array", "mine", "smelter"]);
            process_production(&mut pool, &placed, 4.0_f32, &reg);
            (pool.amount("iron_ore"), pool.amount("iron_plate"))
        };

        let a = run();
        let b = run();
        assert_eq!(a, b, "production must be deterministic for same inputs");
    }

    // ── Chain sustains when fed, scales when short ───────────────────────────

    #[test]
    fn chain_sustains_when_fed_and_scales_when_short() {
        let reg = make_registry_with_power();

        // Full feed: mine produces 10 ore/turn, smelter consumes 2 ore to make 1 plate.
        // After 1 turn with lots of ore: net +8 ore (from mine output) and +1 plate.
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 200.0);
        let placed = buildings(&["solar_array", "mine", "smelter"]);
        let out = process_production(&mut pool, &placed, 100.0_f32, &reg);
        assert!(
            out.building_results
                .iter()
                .all(|r| r.is_full_production() || r.building_type == "solar_array"),
            "expected full production when well-supplied"
        );

        // Now drain ore to force partial production.
        pool.withdraw("iron_ore", pool.amount("iron_ore")); // clear
        pool.deposit("iron_ore", 1.0); // only 1 unit
        let out2 = process_production(&mut pool, &placed, 100.0_f32, &reg);
        let smelt = out2
            .building_results
            .iter()
            .find(|r| r.building_type == "smelter")
            .unwrap();
        assert!(
            smelt.scale < 1.0,
            "smelter should run at partial scale when ore is scarce"
        );
    }

    // ── Brownout is not binary ────────────────────────────────────────────────

    #[test]
    fn brownout_is_continuous_not_binary() {
        // Verify that at ~67% power, output is ~0.67x not 0.
        // solar_array: 100 kW supply.
        // Three smelters × 50 kW each = 150 kW demand.
        // brownout_ratio = 100 / 150 ≈ 0.667, not 0.
        let reg = make_registry_with_power();

        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 100.0);

        let placed = buildings(&["solar_array", "smelter", "smelter", "smelter"]);
        let outcome = process_production(&mut pool, &placed, 100.0_f32, &reg);

        let supply_ratio = outcome.power_grid.supply_ratio();
        assert!(
            supply_ratio > 0.0 && supply_ratio < 1.0,
            "expected partial brownout ratio, got {supply_ratio}"
        );

        for res in &outcome.building_results {
            assert!(
                res.scale > 0.0,
                "building {} should have non-zero output during partial brownout",
                res.building_type
            );
        }
    }

    // ── Maintenance (issue #180) ─────────────────────────────────────────────

    /// Registry with an `advanced_smelter` that requires 0.5 spare_parts / sol
    /// of upkeep and runs a recipe with no other inputs.
    fn make_registry_with_maintenance() -> ContentRegistry {
        let mut reg = ContentRegistry::default();

        reg.insert_building(BuildingDef {
            id: "solar_array".into(),
            name: "Solar Array".into(),
            description: String::new(),
            category: BuildingCategory::Power,
            construction_cost: vec![],
            power_delta: -100.0,
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });

        reg.insert_building(BuildingDef {
            id: "advanced_smelter".into(),
            name: "Advanced Smelter".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 1,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![Ingredient {
                id: "spare_parts".into(),
                quantity: 0.5,
            }],
        });
        reg.insert_recipe(RecipeDef {
            id: "advanced_smelt".into(),
            name: "Advanced Smelt".into(),
            building: "advanced_smelter".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "iron_plate".into(),
                quantity: 1.0,
            }],
            cycle_sols: 1,
            power_draw: 0.0,
        });

        reg
    }

    #[test]
    fn maintenance_short_scales_building_to_zero_when_pool_empty() {
        // Q1=A + Q2=A: a building that needs upkeep and gets none stops.
        let reg = make_registry_with_maintenance();
        let mut pool = ColonyPool::new(); // empty — no spare_parts

        let placed = buildings(&["solar_array", "advanced_smelter"]);
        let outcome = process_production_scaled(
            &mut pool, &placed, 10.0_f32, &reg, 1.0, 1.0, true,
        );

        let smelter = outcome
            .building_results
            .iter()
            .find(|r| r.building_type == "advanced_smelter")
            .expect("advanced_smelter must appear even with maintenance-only tightness");
        assert!(
            smelter.scale.abs() < 1e-9,
            "expected scale 0.0 with empty pool, got {}",
            smelter.scale
        );
        assert!(
            smelter.shortfalls.iter().any(|s| matches!(
                &s.reason,
                ShortfallReason::MaintenanceShort { commodity_id } if commodity_id == "spare_parts"
            )),
            "expected MaintenanceShort(spare_parts) shortfall, got {:?}",
            smelter.shortfalls
        );
        // No iron_plate produced because scale was 0.
        assert!(pool.amount("iron_plate").abs() < 1e-9);
    }

    #[test]
    fn maintenance_consumption_scalar_multiplies_effective_draw() {
        // A 2.0× MaintenanceConsumption scalar doubles the per-sol upkeep, so
        // a pool sized for the neutral rate covers only half the demand.
        let reg = make_registry_with_maintenance();
        let placed = buildings(&["solar_array", "advanced_smelter"]);

        // 0.5 spare_parts base — deposit exactly 0.5.
        let mut pool_normal = ColonyPool::new();
        pool_normal.deposit("spare_parts", 0.5);
        let normal = process_production_scaled(
            &mut pool_normal, &placed, 10.0_f32, &reg, 1.0, 1.0, true,
        );

        let mut pool_harsh = ColonyPool::new();
        pool_harsh.deposit("spare_parts", 0.5);
        let harsh = process_production_scaled(
            &mut pool_harsh, &placed, 10.0_f32, &reg, 1.0, 2.0, true,
        );

        let get_scale = |o: &ProductionStepOutcome, ty: &str| -> f64 {
            o.building_results
                .iter()
                .find(|r| r.building_type == ty)
                .map(|r| r.scale)
                .unwrap()
        };

        let normal_scale = get_scale(&normal, "advanced_smelter");
        let harsh_scale = get_scale(&harsh, "advanced_smelter");

        assert!(
            (normal_scale - 1.0).abs() < 1e-6,
            "neutral scalar should run at full scale, got {normal_scale}"
        );
        assert!(
            (harsh_scale - 0.5).abs() < 1e-6,
            "2× scalar should halve the effective scale, got {harsh_scale}"
        );
    }

    #[test]
    fn maintenance_enabled_false_short_circuits_drain() {
        // Q1=A + master toggle: with maintenance_enabled=false the smelter
        // runs at full scale even when the maintenance commodity is empty.
        let reg = make_registry_with_maintenance();
        let mut pool = ColonyPool::new(); // empty pool

        let placed = buildings(&["solar_array", "advanced_smelter"]);
        let outcome = process_production_scaled(
            &mut pool, &placed, 10.0_f32, &reg, 1.0, 1.0, false,
        );

        let smelter = outcome
            .building_results
            .iter()
            .find(|r| r.building_type == "advanced_smelter")
            .unwrap();
        assert!(
            (smelter.scale - 1.0).abs() < 1e-6,
            "master maintenance toggle off should ignore upkeep, got scale {}",
            smelter.scale
        );
        assert!(
            !smelter.shortfalls.iter().any(|s| matches!(
                s.reason,
                ShortfallReason::MaintenanceShort { .. }
            )),
            "no MaintenanceShort should fire when maintenance is disabled"
        );
        // Recipe output still runs at scale 1.
        assert!((pool.amount("iron_plate") - 1.0).abs() < 1e-6);
        // No spare_parts drained.
        assert!(pool.amount("spare_parts").abs() < 1e-9);
    }

    #[test]
    fn shared_commodity_reports_as_input_short_not_maintenance() {
        // Backwards-compat guard: when a commodity appears in BOTH the recipe
        // inputs and the maintenance list, a tightness on that commodity must
        // still report as InputShort (existing UI/interrupt contracts).
        let mut reg = ContentRegistry::default();

        reg.insert_building(BuildingDef {
            id: "solar_array".into(),
            name: "Solar Array".into(),
            description: String::new(),
            category: BuildingCategory::Power,
            construction_cost: vec![],
            power_delta: -100.0,
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });

        reg.insert_building(BuildingDef {
            id: "recycler".into(),
            name: "Recycler".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 1,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            // Same commodity as the recipe input.
            maintenance: vec![Ingredient {
                id: "scrap".into(),
                quantity: 0.5,
            }],
        });
        reg.insert_recipe(RecipeDef {
            id: "recycle".into(),
            name: "Recycle".into(),
            building: "recycler".into(),
            inputs: vec![Ingredient {
                id: "scrap".into(),
                quantity: 1.0,
            }],
            outputs: vec![],
            cycle_sols: 1,
            power_draw: 0.0,
        });

        let mut pool = ColonyPool::new();
        pool.deposit("scrap", 0.3); // less than combined demand of 1.5

        let placed = buildings(&["solar_array", "recycler"]);
        let outcome = process_production_scaled(
            &mut pool, &placed, 10.0_f32, &reg, 1.0, 1.0, true,
        );

        let recycler = outcome
            .building_results
            .iter()
            .find(|r| r.building_type == "recycler")
            .unwrap();
        assert!(
            recycler.shortfalls.iter().any(|s| matches!(
                &s.reason,
                ShortfallReason::InputShort { commodity_id } if commodity_id == "scrap"
            )),
            "shared commodity tightness must report InputShort, got {:?}",
            recycler.shortfalls
        );
        assert!(
            !recycler.shortfalls.iter().any(|s| matches!(
                s.reason,
                ShortfallReason::MaintenanceShort { .. }
            )),
            "shared commodity must NOT report as MaintenanceShort"
        );
    }

    #[test]
    fn empty_maintenance_list_is_free() {
        // Q4: authored empty maintenance means no drain regardless of pool state.
        let reg = make_registry_with_power(); // no maintenance authored anywhere
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 100.0);

        let placed = buildings(&["solar_array", "mine", "smelter"]);
        let outcome = process_production_scaled(
            &mut pool, &placed, 10.0_f32, &reg, 1.0, 1.0, true,
        );

        for res in &outcome.building_results {
            assert!(
                !res.shortfalls.iter().any(|s| matches!(
                    s.reason,
                    ShortfallReason::MaintenanceShort { .. }
                )),
                "no MaintenanceShort should fire when nothing has upkeep authored (building: {})",
                res.building_type
            );
        }
    }
}
