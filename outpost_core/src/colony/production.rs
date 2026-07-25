//! Per-turn production resolution for colony buildings.
//!
//! Each turn, operational buildings scale their output by
//! `min(input_ratio, power_ratio, labor_ratio)` — a continuous scalar, not a
//! binary on/off step. Shortfalls cause partial production, never a panic.
//!
//! See `docs/DESIGN.md §5, §7`. The C#
//! `ColonyTurnProcessor.ProcessProduction()` is the behavioural spec;
//! the key improvement here is that scaling is continuous.
//!
//! # Concurrent (multi-function) recipes
//!
//! A building type can run **at most one** pick-one recipe (the
//! `active_recipes`-selected kind, issue #166) plus **any number** of
//! always-on [`RecipeDef::concurrent`] recipes, every turn, simultaneously —
//! the "true simultaneous multi-output buildings" mechanism deferred from
//! playtest feedback's multi-function starter building idea (a combined
//! colony HQ / power-atmosphere-water building, etc.). Concurrent recipes
//! never participate in [`crate::colony::Colony::active_recipes`]'s
//! selection — they simply always run.
//!
//! All of a building instance's simultaneously-running recipes (pick-one +
//! every concurrent one) share **one** combined scale factor this turn:
//! their inputs, maintenance draws, and deposit-gated outputs are pooled
//! into a single demand computation (mirroring how recipe inputs and
//! maintenance already combine, issue #180), rather than each recipe
//! getting its own independent scale. This keeps the "one building = one
//! operational state this turn" mental model intact — a multi-function
//! building throttles all its outputs together under a shared power/labor/
//! input constraint, rather than one function silently continuing at full
//! output while another starves.

use crate::content::types::Ingredient;
use crate::content::{BuildingCategory, ContentRegistry, RecipeDef};

use super::stores::ColonyStores;

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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductionShortfall {
    /// Human-readable description of what was short.
    pub reason: ShortfallReason,
    /// The scale factor that was actually applied (`< 1.0` when short).
    pub effective_scale: f64,
}

/// Category of shortfall limiting a building's production.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
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
    /// A deposit-gated extraction recipe ran below full output because no
    /// matching deposit (or only a low-richness one) was found at the
    /// colony's site/body (issue #239). `effective_scale` on the enclosing
    /// [`ProductionShortfall`] distinguishes a total absence (`0.0`) from a
    /// merely scarce deposit (`0.0 < scale < 1.0`).
    DepositShort {
        /// The deposit-gated commodity id that was the tightest constraint.
        commodity_id: String,
    },
}

/// Outcome of one building's production attempt this turn.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildingProductionResult {
    /// Content-pack key of the building type.
    pub building_type: String,
    /// The pick-one recipe that ran (or was attempted), if the building has
    /// one — empty string when the building has no pick-one recipe (either
    /// no recipes at all, or every authored recipe is [`RecipeDef::concurrent`]).
    pub recipe_id: String,
    /// Every additional `concurrent` recipe that ran alongside `recipe_id`
    /// this turn (issue: multi-function starter buildings / "true
    /// simultaneous multi-output buildings"). Empty for an ordinary
    /// single-recipe building — this field is purely additive so existing
    /// `recipe_id`-only consumers (UI, `last_production` lookups) keep
    /// working unchanged.
    #[serde(default)]
    pub concurrent_recipe_ids: Vec<String>,
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
    /// Every `RecipeDef::concurrent` recipe this building type has — always
    /// runs alongside `recipe`, sharing the same `scale`. See this module's
    /// doc comment.
    concurrent_recipes: Vec<&'a RecipeDef>,
    building_category: BuildingCategory,
    maintenance: &'a [Ingredient],
    /// Effective per-sol maintenance multiplier
    /// (`MaintenanceConsumption` scalar; `0.0` when disabled).
    maintenance_multiplier: f64,
    scale: f64,
    shortfalls: Vec<ProductionShortfall>,
}

/// Classify a recipe output into a [`crate::system::YieldCategory`] (issue
/// #184), so a body's per-category [`crate::system::BodyModifier`]s apply
/// to the right slice of production.
///
/// `BuildingCategory::Power`/`Research` map directly to `PowerOutput`/
/// `ScienceYield` — those categories are unambiguous regardless of what
/// they output. Everything else is split by the *output commodity's* own
/// `category` string rather than the building's, since `Extraction`/
/// `Processing` buildings span both food chains (hydroponic bays) and
/// industrial chains (ore mines, refineries) under the same
/// `BuildingCategory` — commodities tagged `consumable` count as food,
/// everything else defaults to `IndustryYield`.
fn yield_category_for(
    building_category: &BuildingCategory,
    commodity_category: Option<&str>,
) -> crate::system::YieldCategory {
    use crate::system::YieldCategory;
    match building_category {
        BuildingCategory::Power => YieldCategory::PowerOutput,
        BuildingCategory::Research => YieldCategory::ScienceYield,
        _ if commodity_category == Some("consumable") => YieldCategory::FoodYield,
        _ => YieldCategory::IndustryYield,
    }
}

/// Map a [`crate::system::YieldCategory`] to the `TechEffect::Bonus`
/// `category` string content authors use for the matching tech-tree concept
/// (issue #248) — e.g. `content/base/tech.yaml`'s `power_generation` bonus
/// techs boost `PowerOutput` recipes. This is the sole place that
/// reconciles the tech tree's free-form bonus-category vocabulary
/// (`power_generation`, `research_output`, `food_production`,
/// `production_efficiency`) with the structural [`crate::system::YieldCategory`]
/// enum production.rs already classifies every output by.
fn tech_bonus_category_key(category: crate::system::YieldCategory) -> &'static str {
    use crate::system::YieldCategory;
    match category {
        YieldCategory::PowerOutput => "power_generation",
        YieldCategory::ScienceYield => "research_output",
        YieldCategory::FoodYield => "food_production",
        YieldCategory::IndustryYield => "production_efficiency",
    }
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
/// * `stores`          — mutable colony stores, commodity + resource (modified in-place)
/// * `buildings`       — slice of `(building_type, slot_cost)` pairs for placed buildings
/// * `labor_available` — labour units available this turn (from `PopulationPool::available_labor`)
/// * `registry`        — content registry for looking up `BuildingDef` and `RecipeDef`
pub fn process_production(
    stores: &mut ColonyStores<'_>,
    buildings: &[(String, u32)],
    labor_available: f32,
    registry: &ContentRegistry,
) -> ProductionStepOutcome {
    process_production_scaled(
        stores,
        buildings,
        labor_available,
        registry,
        1.0,
        1.0,
        true,
        1.0,
        &std::collections::HashMap::new(),
        &[],
        None,
        &crate::modifier::ModifierAccumulator::new(),
        &crate::modifier::DifficultyScalar::new(),
    )
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
///
/// `productivity_multiplier` scales every recipe **output** deposit (issue
/// #163) — used to fold in the colony's habitability modifier. Inputs and
/// maintenance draws are unaffected so worlds that are hard to live on still
/// consume the same feedstock but yield less product.
///
/// `category_modifiers` (issue #184) applies an *additional*, per-output
/// multiplicative factor on top of `productivity_multiplier`, resolved per
/// [`crate::system::YieldCategory`] via [`yield_category_for`] — a body can
/// author an elevated `power_output` modifier, say, without that also
/// inflating its food or science yield.
///
/// `modifier_accumulator`/`difficulty_scalar` (issue #248) resolve a third,
/// independent per-output multiplicative factor via
/// [`crate::modifier::resolve`], keyed by [`tech_bonus_category_key`] —
/// this is what gives `TechEffect::Bonus` techs (e.g. `power_generation`,
/// `research_output`, `food_production`, `production_efficiency`) a real
/// numeric effect, following the stacking formula from `docs/DESIGN.md
/// §7A`: `effective = base × (1 + Σ tech_bonuses_in_category) ×
/// difficulty_scalar`. Pass empty/default instances for the pre-#248
/// no-tech-bonus-applies case (see [`process_production`]).
///
/// `deposit_richness` (issue #239) maps commodity id → richness/abundance
/// in `[0.0, 1.0]` for whatever deposits are available at the colony's
/// site/body. Only recipes with at least one output in
/// [`crate::map::VEIN_COMMODITIES`] (the curated raw-material list #232
/// guarantees placement for) are deposit-gated; every other recipe is
/// unaffected.
///
/// Pass `None` when the colony has **no spatial placement to check against
/// at all** (e.g. founded via the bare `Command::FoundColony` test/fixture
/// path with no hex or body link) — gating is inert, matching
/// [`process_production`]. Pass `Some(map)` — even an *empty* map — whenever
/// the colony genuinely has a site/body: a real location with zero deposits
/// on record is a meaningfully different case from "no placement data
/// exists," and must still gate (missing map entries mean `0.0` richness).
#[allow(
    clippy::too_many_arguments,
    clippy::implicit_hasher,
    clippy::too_many_lines
)]
pub fn process_production_scaled(
    stores: &mut ColonyStores<'_>,
    buildings: &[(String, u32)],
    labor_available: f32,
    registry: &ContentRegistry,
    power_scalar: f32,
    maintenance_scalar: f32,
    maintenance_enabled: bool,
    productivity_multiplier: f32,
    active_recipes: &std::collections::HashMap<String, String>,
    category_modifiers: &[crate::system::BodyModifier],
    deposit_richness: Option<&std::collections::HashMap<String, f32>>,
    modifier_accumulator: &crate::modifier::ModifierAccumulator,
    difficulty_scalar: &crate::modifier::DifficultyScalar,
) -> ProductionStepOutcome {
    // ── Step 1: build power grid ─────────────────────────────────────────────
    let power_grid = compute_power_grid_scaled(buildings, registry, power_scalar, active_recipes);
    let brownout_ratio = power_grid.supply_ratio();

    // ── Step 2: compute labor ratio ──────────────────────────────────────────
    let labor_demanded: f32 = labor_demanded(buildings.iter().map(|(bt, _)| bt.as_str()), registry);

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
    // Pass B: apply all scaled changes to the stores.
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
        let recipe = recipe_for_building(building_type, active_recipes, registry);
        let concurrent_recipes = concurrent_recipes_for_building(building_type, registry);
        let has_any_recipe = recipe.is_some() || !concurrent_recipes.is_empty();
        let has_maintenance = maintenance_enabled && !bdef.maintenance.is_empty();

        // Buildings with neither a recipe nor an active maintenance list stay
        // out of the production pass entirely (unchanged pre-#180 behaviour).
        if !has_any_recipe && !has_maintenance {
            continue;
        }

        // Combined per-commodity demand (pick-one recipe inputs + every
        // concurrent recipe's inputs + maintenance) — see this module's doc
        // comment on why a multi-function building shares one scale.
        let (input_ratio, tight_commodity, tight_is_maintenance) = compute_effective_input_ratio(
            stores,
            recipe,
            &concurrent_recipes,
            if has_maintenance {
                bdef.maintenance.as_slice()
            } else {
                &[]
            },
            maintenance_multiplier,
        );

        // Power/labor ratios only apply to recipe-running buildings. A pure
        // maintenance-only building doesn't consume labour or drive brownouts.
        let (power_ratio, applies_labor) = if has_any_recipe {
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

        // Deposit gating (issue #239) — only applies to deposit-gated
        // recipes; inert (ratio 1.0) for everything else.
        let (deposit_ratio, deposit_tight) =
            compute_deposit_ratio(recipe, &concurrent_recipes, deposit_richness);

        // Overall scale factor.
        let scale = input_ratio
            .min(power_ratio)
            .min(effective_labor_ratio)
            .min(deposit_ratio)
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
        if deposit_ratio < 1.0 - 1e-9 {
            shortfalls.push(ProductionShortfall {
                reason: ShortfallReason::DepositShort {
                    commodity_id: deposit_tight.unwrap_or_default(),
                },
                effective_scale: deposit_ratio,
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
            concurrent_recipes,
            building_category: bdef.category.clone(),
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
    let output_multiplier = f64::from(productivity_multiplier.max(0.0));
    let mut building_results: Vec<BuildingProductionResult> = Vec::new();
    for p in pending {
        // Every simultaneously-running recipe (the pick-one recipe, if any,
        // plus every concurrent one) shares `p.scale` — see this module's
        // doc comment.
        if p.scale > 1e-9 {
            let running_recipes = p
                .recipe
                .into_iter()
                .chain(p.concurrent_recipes.iter().copied());
            for recipe in running_recipes {
                for ingredient in &recipe.inputs {
                    stores.withdraw(&ingredient.id, ingredient.quantity * p.scale);
                }
                for ingredient in &recipe.outputs {
                    let commodity_category = registry
                        .commodity(&ingredient.id)
                        .map(|c| c.category.as_str());
                    let category = yield_category_for(&p.building_category, commodity_category);
                    let category_mult = f64::from(crate::system::category_modifier(
                        category_modifiers,
                        category,
                    ));
                    let tech_mult = f64::from(crate::modifier::resolve(
                        1.0,
                        &crate::modifier::ModifiableQuantity::ProductionRate(
                            tech_bonus_category_key(category).to_string(),
                        ),
                        modifier_accumulator,
                        difficulty_scalar,
                    ));
                    stores.deposit(
                        &ingredient.id,
                        ingredient.quantity
                            * p.scale
                            * output_multiplier
                            * category_mult
                            * tech_mult,
                    );
                }
            }
            for ingredient in p.maintenance {
                stores.withdraw(
                    &ingredient.id,
                    ingredient.quantity * p.maintenance_multiplier * p.scale,
                );
            }
        }
        let recipe_id = p.recipe.map(|r| r.id.clone()).unwrap_or_default();
        let concurrent_recipe_ids = p.concurrent_recipes.iter().map(|r| r.id.clone()).collect();
        building_results.push(BuildingProductionResult {
            building_type: p.building_type.to_owned(),
            recipe_id,
            concurrent_recipe_ids,
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
    active_recipes: &std::collections::HashMap<String, String>,
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
        // Recipe power draw adds to demand — the pick-one recipe (if any)
        // plus every concurrent recipe running alongside it.
        let recipe = recipe_for_building(building_type, active_recipes, registry);
        let concurrent_recipes = concurrent_recipes_for_building(building_type, registry);
        for r in recipe.into_iter().chain(concurrent_recipes) {
            demand += r.power_draw * mul;
        }
    }

    PowerGrid { capacity, demand }
}

/// Compute the deposit-availability scale factor for a recipe (issue #239).
///
/// Only recipes with at least one output in [`crate::map::VEIN_COMMODITIES`]
/// are deposit-gated; everything else (including recipes with no deposit-
/// tracked output at all, e.g. a water well) returns `(1.0, None)`
/// unconditionally.
///
/// `deposit_richness` is `None` when the colony has **no spatial placement
/// to check against at all** (e.g. founded via the bare `Command::FoundColony`
/// test/fixture path) — gating is inert in that case. It is `Some(map)`
/// (possibly an *empty* map — a real hex/body with zero deposits on record
/// is a meaningfully different case from "no placement data exists at
/// all") whenever the colony genuinely has a site or body to check;
/// missing entries in that map mean "no matching deposit here" (`0.0`
/// richness), not "gating doesn't apply."
///
/// When gating applies, the *worst* richness among the recipe's
/// deposit-gated outputs sets the ratio: total absence (`0.0` richness,
/// i.e. no matching deposit at all) drops the ratio to `0.0`; presence of
/// any deposit guarantees a `0.5` floor, scaling linearly up to `1.0` at
/// richness `1.0` — so a guaranteed-placed but low-richness deposit
/// (#232's coverage guarantee) still produces something, while richness
/// genuinely matters rather than being a pure presence/absence toggle.
fn compute_deposit_ratio(
    recipe: Option<&RecipeDef>,
    concurrent: &[&RecipeDef],
    deposit_richness: Option<&std::collections::HashMap<String, f32>>,
) -> (f64, Option<String>) {
    let running = recipe.into_iter().chain(concurrent.iter().copied());
    let vein_outputs: Vec<&str> = running
        .flat_map(|r| r.outputs.iter())
        .map(|o| o.id.as_str())
        .filter(|id| crate::map::VEIN_COMMODITIES.contains(id))
        .collect();
    if vein_outputs.is_empty() {
        return (1.0, None);
    }
    let Some(deposit_richness) = deposit_richness else {
        return (1.0, None);
    };

    let mut worst_commodity = vein_outputs[0];
    let mut worst_richness = f32::MAX;
    for id in &vein_outputs {
        let richness = deposit_richness.get(*id).copied().unwrap_or(0.0);
        if richness < worst_richness {
            worst_richness = richness;
            worst_commodity = id;
        }
    }

    if worst_richness <= 0.0 {
        return (0.0, Some(worst_commodity.to_string()));
    }
    let ratio = f64::from(0.5 + worst_richness.clamp(0.0, 1.0) * 0.5);
    if ratio < 1.0 - 1e-9 {
        (ratio, Some(worst_commodity.to_string()))
    } else {
        (ratio, None)
    }
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
    stores: &ColonyStores<'_>,
    recipe: Option<&RecipeDef>,
    concurrent: &[&RecipeDef],
    maintenance: &[Ingredient],
    maintenance_multiplier: f64,
) -> (f64, Option<String>, bool) {
    // Merged (id, quantity, has_recipe_demand) list, preserving deterministic
    // order: recipe inputs first (summed across the pick-one recipe and
    // every concurrent recipe — a multi-function building's simultaneous
    // demands are pooled, not tracked independently), then maintenance
    // entries (with dedup).
    let mut demands: Vec<(String, f64, bool)> = Vec::new();

    for r in recipe.into_iter().chain(concurrent.iter().copied()) {
        for ing in &r.inputs {
            if ing.quantity <= 0.0 {
                continue;
            }
            if let Some(existing) = demands.iter_mut().find(|d| d.0 == ing.id) {
                existing.1 += ing.quantity;
            } else {
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
        let available = stores.amount(id);
        let r = (available / *qty).min(1.0);
        if r < ratio {
            ratio = r;
            tight = Some(id.clone());
            tight_is_maintenance = !*has_recipe_demand;
        }
    }

    (ratio.max(0.0), tight, tight_is_maintenance)
}

/// Return the first **non-concurrent** (pick-one-eligible) recipe whose
/// `building` field matches `building_type`. Deterministic default recipe
/// for a building type when no active selection applies: the
/// lexicographically smallest recipe id among matches.
///
/// [`RecipeDef::concurrent`] recipes are excluded — they always run
/// regardless of this selection (see [`concurrent_recipes_for_building`]),
/// so they must never also be picked as *the* pick-one default (that would
/// run them twice).
///
/// `ContentRegistry` stores recipes in a `HashMap`, so iteration order is
/// otherwise unspecified — for a building with only one recipe this is moot,
/// but as of issue #166 some buildings expose more than one, and picking an
/// arbitrary hash-order "first" would make the default unpredictable across
/// runs.
fn first_recipe_for_building<'r>(
    building_type: &str,
    registry: &'r ContentRegistry,
) -> Option<&'r RecipeDef> {
    registry
        .recipes()
        .filter(|r| r.building == building_type && !r.concurrent)
        .min_by(|a, b| a.id.cmp(&b.id))
}

/// Resolve the pick-one recipe a building instance actually runs (issue
/// #166).
///
/// If `active_recipes` names a recipe for this `building_type` and that
/// recipe exists, actually belongs to this building, and is **not**
/// [`RecipeDef::concurrent`], it wins; otherwise falls back to
/// [`first_recipe_for_building`] (the pre-#166 deterministic default — the
/// first authored non-concurrent recipe for the type). This keeps
/// single-recipe buildings working with no player action needed, and a
/// building whose every recipe is `concurrent` correctly returns `None`
/// here (its recipes still run — see [`concurrent_recipes_for_building`]).
pub(crate) fn recipe_for_building<'r, S: std::hash::BuildHasher>(
    building_type: &str,
    active_recipes: &std::collections::HashMap<String, String, S>,
    registry: &'r ContentRegistry,
) -> Option<&'r RecipeDef> {
    if let Some(recipe_id) = active_recipes.get(building_type) {
        if let Some(recipe) = registry
            .recipes()
            .find(|r| &r.id == recipe_id && r.building == building_type && !r.concurrent)
        {
            return Some(recipe);
        }
    }
    first_recipe_for_building(building_type, registry)
}

/// Every [`RecipeDef::concurrent`] recipe authored for `building_type` —
/// these always run alongside whatever [`recipe_for_building`] resolves
/// (if anything), every turn, with no player selection needed. See this
/// module's doc comment.
pub(crate) fn concurrent_recipes_for_building<'r>(
    building_type: &str,
    registry: &'r ContentRegistry,
) -> Vec<&'r RecipeDef> {
    let mut recipes: Vec<&RecipeDef> = registry
        .recipes()
        .filter(|r| r.building == building_type && r.concurrent)
        .collect();
    // Deterministic ordering, same discipline as `first_recipe_for_building`.
    recipes.sort_by(|a, b| a.id.cmp(&b.id));
    recipes
}

/// A building type's combined per-cycle input/output footprint at **full
/// output** (issue #272).
///
/// Until now a building's "0-N inputs, 0-N outputs" profile only existed
/// implicitly — you had to mentally sum whichever recipes happen to run. This is
/// that sum, made explicit for the UI and for content authors.
///
/// **Nominal, not actual.** These are the authored recipe quantities, unscaled:
/// what the building would move at `scale == 1.0`. Last turn's *real* throughput
/// is `nominal × scale`, where `scale` comes from
/// [`BuildingProductionResult`]. That is deliberate — issue #272 gap 3 asked for
/// a *declared* footprint an author can check against their intent, which has to
/// be independent of whatever a particular colony managed this turn — but it
/// means a consumer showing these figures next to a shortfall must say which it
/// is showing, or the two will appear to contradict each other.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BuildingIoSummary {
    /// Recipe ids that contributed, pick-one first then concurrent ones in id
    /// order. Empty for a building with no recipes at all.
    pub recipe_ids: Vec<String>,
    /// Commodities consumed per cycle, merged across recipes, sorted by id.
    pub inputs: Vec<(String, f64)>,
    /// Commodities produced per cycle, merged across recipes, sorted by id.
    pub outputs: Vec<(String, f64)>,
}

impl BuildingIoSummary {
    /// `true` when this building neither consumes nor produces anything — a
    /// pure storage or habitat structure.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty() && self.outputs.is_empty()
    }
}

/// Sum the full-output per-cycle flows of every recipe a building instance
/// runs (issue #272).
///
/// That is the resolved pick-one recipe (`active_recipes`' selection, else the
/// deterministic default) **plus** every [`RecipeDef::concurrent`] recipe — the
/// same set the production step runs on one shared scale factor, so the summary
/// covers the building's whole function rather than one arbitrary recipe.
///
/// The quantities are **nominal**: unscaled authored rates, not what the
/// building achieved last turn. See [`BuildingIoSummary`].
///
/// A commodity appearing in more than one running recipe is **merged**, not
/// listed twice: two concurrent recipes each producing 5 power report 10, which
/// is what actually lands in the pool. Note that a commodity appearing as both
/// an input and an output stays in both lists rather than being netted —
/// throughput and net change are different questions, and the caller may want
/// either.
///
/// Quantities are per *cycle*, matching `RecipeDef`, not per sol — divide by
/// `cycle_sols` for a rate (see [`crate::balance`]).
#[must_use]
pub fn building_io_summary<S: std::hash::BuildHasher>(
    building_type: &str,
    active_recipes: &std::collections::HashMap<String, String, S>,
    registry: &ContentRegistry,
) -> BuildingIoSummary {
    let pick_one = recipe_for_building(building_type, active_recipes, registry);
    let concurrent = concurrent_recipes_for_building(building_type, registry);

    let mut recipe_ids = Vec::new();
    let mut inputs: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
    let mut outputs: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();

    for recipe in pick_one.into_iter().chain(concurrent) {
        recipe_ids.push(recipe.id.clone());
        for i in &recipe.inputs {
            *inputs.entry(i.id.clone()).or_default() += i.quantity;
        }
        for o in &recipe.outputs {
            *outputs.entry(o.id.clone()).or_default() += o.quantity;
        }
    }

    BuildingIoSummary {
        recipe_ids,
        inputs: inputs.into_iter().collect(),
        outputs: outputs.into_iter().collect(),
    }
}

/// Returns true if there is at least one recipe (pick-one or concurrent)
/// for the given building type.
fn has_recipe(building_type: &str, registry: &ContentRegistry) -> bool {
    first_recipe_for_building(building_type, registry).is_some()
        || !concurrent_recipes_for_building(building_type, registry).is_empty()
}

/// Total worker slots demanded by the given buildings — the "jobs offered" a
/// colony's labour ratio is divided against.
///
/// Only buildings with at least one recipe count: a facility with nothing to
/// produce asks for nobody. `pub(crate)` because the colony-screen query
/// (issue #305) reports employed/unemployed from this same number — computing
/// it twice would let the readout drift from what actually gates production.
pub(crate) fn labor_demanded<'a, I>(building_types: I, registry: &ContentRegistry) -> f32
where
    I: IntoIterator<Item = &'a str>,
{
    building_types
        .into_iter()
        .filter_map(|bt| registry.building(bt))
        .filter(|bd| has_recipe(bd.id.as_str(), registry))
        .map(|bd| {
            #[allow(clippy::cast_precision_loss)]
            {
                bd.worker_slots as f32
            }
        })
        .sum()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::{ColonyPool, ColonyResourcePool};
    use crate::content::types::{BuildingCategory, BuildingDef, Ingredient, RecipeDef};
    use crate::content::ContentRegistry;

    // ── Bare-pool test shims (issue #304) ────────────────────────────────────
    //
    // These deliberately **shadow** the module's `process_production` /
    // `process_production_scaled`, which now take a `ColonyStores` view over a
    // colony's commodity pool *and* its resource pool.
    //
    // Every test below exercises a commodity chain against a registry it builds
    // itself, and none of those registries declare a colony resource — so a
    // throwaway resource pool is provably equivalent to the real stores for
    // them, and keeping the old call shape avoids rewriting ~35 assertions that
    // have nothing to do with this change. Tests that *do* care about resource
    // routing call `super::process_production_scaled` explicitly.

    fn process_production(
        pool: &mut ColonyPool,
        buildings: &[(String, u32)],
        labor_available: f32,
        registry: &ContentRegistry,
    ) -> ProductionStepOutcome {
        let mut resources = ColonyResourcePool::new();
        super::process_production(
            &mut ColonyStores::new(pool, &mut resources, registry),
            buildings,
            labor_available,
            registry,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn process_production_scaled(
        pool: &mut ColonyPool,
        buildings: &[(String, u32)],
        labor_available: f32,
        registry: &ContentRegistry,
        power_scalar: f32,
        maintenance_scalar: f32,
        maintenance_enabled: bool,
        productivity_multiplier: f32,
        active_recipes: &std::collections::HashMap<String, String>,
        category_modifiers: &[crate::system::BodyModifier],
        deposit_richness: Option<&std::collections::HashMap<String, f32>>,
        modifier_accumulator: &crate::modifier::ModifierAccumulator,
        difficulty_scalar: &crate::modifier::DifficultyScalar,
    ) -> ProductionStepOutcome {
        let mut resources = ColonyResourcePool::new();
        super::process_production_scaled(
            &mut ColonyStores::new(pool, &mut resources, registry),
            buildings,
            labor_available,
            registry,
            power_scalar,
            maintenance_scalar,
            maintenance_enabled,
            productivity_multiplier,
            active_recipes,
            category_modifiers,
            deposit_richness,
            modifier_accumulator,
            difficulty_scalar,
        )
    }

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
            concurrent: false,
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
            concurrent: false,
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
            concurrent: false,
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
            &mut pool,
            &placed,
            10.0_f32,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
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
            &mut pool_normal,
            &placed,
            10.0_f32,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        let mut pool_harsh = ColonyPool::new();
        pool_harsh.deposit("spare_parts", 0.5);
        let harsh = process_production_scaled(
            &mut pool_harsh,
            &placed,
            10.0_f32,
            &reg,
            1.0,
            2.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
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
            &mut pool,
            &placed,
            10.0_f32,
            &reg,
            1.0,
            1.0,
            false,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
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
            !smelter
                .shortfalls
                .iter()
                .any(|s| matches!(s.reason, ShortfallReason::MaintenanceShort { .. })),
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
            concurrent: false,
        });

        let mut pool = ColonyPool::new();
        pool.deposit("scrap", 0.3); // less than combined demand of 1.5

        let placed = buildings(&["solar_array", "recycler"]);
        let outcome = process_production_scaled(
            &mut pool,
            &placed,
            10.0_f32,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
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
            !recycler
                .shortfalls
                .iter()
                .any(|s| matches!(s.reason, ShortfallReason::MaintenanceShort { .. })),
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
            &mut pool,
            &placed,
            10.0_f32,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        for res in &outcome.building_results {
            assert!(
                !res.shortfalls
                    .iter()
                    .any(|s| matches!(s.reason, ShortfallReason::MaintenanceShort { .. })),
                "no MaintenanceShort should fire when nothing has upkeep authored (building: {})",
                res.building_type
            );
        }
    }

    // ── Habitability-driven productivity multiplier (issue #163) ─────────────

    #[test]
    fn productivity_multiplier_scales_outputs_only() {
        // With productivity_multiplier = 1.25 the smelter should still consume
        // 2 ore (unchanged input) but deposit 1.25 iron_plate. Inputs are
        // preserved so harsh worlds still consume feedstock at the base rate.
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 10.0);

        let placed = buildings(&["solar_array", "smelter"]);
        let outcome = process_production_scaled(
            &mut pool,
            &placed,
            100.0_f32,
            &reg,
            1.0,
            1.0,
            true,
            1.25,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        let smelt = outcome
            .building_results
            .iter()
            .find(|r| r.building_type == "smelter")
            .unwrap();
        assert!(
            smelt.is_full_production(),
            "smelter shortfalls: {:?}",
            smelt.shortfalls
        );
        // Input drain unchanged: 10 - 2 = 8 ore.
        assert!(
            (pool.amount("iron_ore") - 8.0).abs() < 1e-6,
            "expected 8.0 ore left, got {}",
            pool.amount("iron_ore")
        );
        // Output multiplied: 1.0 * 1.25 = 1.25 iron_plate.
        assert!(
            (pool.amount("iron_plate") - 1.25).abs() < 1e-6,
            "expected 1.25 iron_plate, got {}",
            pool.amount("iron_plate")
        );
    }

    // ── Per-category body modifiers (issue #184) ──────────────────────────────

    #[test]
    fn category_modifier_stacks_multiplicatively_on_matching_category() {
        // iron_plate is an IndustryYield output (smelter is Processing, and
        // iron_plate isn't a `consumable` commodity) — an IndustryYield
        // BodyModifier should multiply on top of productivity_multiplier.
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 10.0);

        let placed = buildings(&["solar_array", "smelter"]);
        let category_modifiers = [crate::system::BodyModifier {
            category: crate::system::YieldCategory::IndustryYield,
            multiplier: 2.0,
        }];
        process_production_scaled(
            &mut pool,
            &placed,
            100.0_f32,
            &reg,
            1.0,
            1.0,
            true,
            1.25,
            &std::collections::HashMap::new(),
            &category_modifiers,
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        // 1.0 base output * 1.25 productivity * 2.0 industry modifier = 2.5.
        assert!(
            (pool.amount("iron_plate") - 2.5).abs() < 1e-6,
            "expected 2.5 iron_plate (1.25 productivity x 2.0 industry_yield), got {}",
            pool.amount("iron_plate")
        );
    }

    #[test]
    fn category_modifier_does_not_apply_to_a_different_category() {
        // A ScienceYield modifier must not affect the smelter's IndustryYield
        // output — modifiers are scoped per category, not colony-wide.
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 10.0);

        let placed = buildings(&["solar_array", "smelter"]);
        let category_modifiers = [crate::system::BodyModifier {
            category: crate::system::YieldCategory::ScienceYield,
            multiplier: 5.0,
        }];
        process_production_scaled(
            &mut pool,
            &placed,
            100.0_f32,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &category_modifiers,
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        assert!(
            (pool.amount("iron_plate") - 1.0).abs() < 1e-6,
            "ScienceYield modifier should not affect IndustryYield output, got {}",
            pool.amount("iron_plate")
        );
    }

    #[test]
    fn tech_bonus_scales_matching_category_output() {
        // iron_plate is IndustryYield (smelter is Processing, iron_plate
        // isn't a `consumable` commodity) — a `production_efficiency`
        // TechEffect::Bonus should multiply on top of the base output
        // (issue #248), following the same category-keyed pattern
        // `category_modifier_stacks_multiplicatively_on_matching_category`
        // already proves for body modifiers.
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 10.0);

        let placed = buildings(&["solar_array", "smelter"]);
        let mut accum = crate::modifier::ModifierAccumulator::new();
        accum.add(crate::modifier::ModifierDescriptor::new(
            crate::modifier::ModifiableQuantity::ProductionRate("production_efficiency".into()),
            "structural_composites",
            0.25,
        ));
        process_production_scaled(
            &mut pool,
            &placed,
            100.0_f32,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            None,
            &accum,
            &crate::modifier::DifficultyScalar::new(),
        );

        // 1.0 base output * (1.0 + 0.25 tech bonus) = 1.25.
        assert!(
            (pool.amount("iron_plate") - 1.25).abs() < 1e-6,
            "expected 1.25 iron_plate (+25% production_efficiency tech bonus), got {}",
            pool.amount("iron_plate")
        );
    }

    #[test]
    fn tech_bonus_does_not_apply_to_a_different_category() {
        // A `research_output` bonus must not affect the smelter's
        // IndustryYield output — tech bonuses are scoped per category, same
        // discipline as body category_modifiers.
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 10.0);

        let placed = buildings(&["solar_array", "smelter"]);
        let mut accum = crate::modifier::ModifierAccumulator::new();
        accum.add(crate::modifier::ModifierDescriptor::new(
            crate::modifier::ModifiableQuantity::ProductionRate("research_output".into()),
            "theoretical_physics",
            0.50,
        ));
        process_production_scaled(
            &mut pool,
            &placed,
            100.0_f32,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            None,
            &accum,
            &crate::modifier::DifficultyScalar::new(),
        );

        assert!(
            (pool.amount("iron_plate") - 1.0).abs() < 1e-6,
            "research_output tech bonus should not affect IndustryYield output, got {}",
            pool.amount("iron_plate")
        );
    }

    #[test]
    fn productivity_multiplier_below_one_reduces_outputs() {
        // A harsh world (0.75×) yields 25 % less product per turn.
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 10.0);

        let placed = buildings(&["solar_array", "smelter"]);
        process_production_scaled(
            &mut pool,
            &placed,
            100.0_f32,
            &reg,
            1.0,
            1.0,
            true,
            0.75,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );
        assert!(
            (pool.amount("iron_plate") - 0.75).abs() < 1e-6,
            "expected 0.75 iron_plate at 0.75× productivity, got {}",
            pool.amount("iron_plate")
        );
    }

    /// A building hosting two authored recipes (issue #166).
    fn make_registry_with_two_recipes() -> ContentRegistry {
        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "refinery".into(),
            name: "Refinery".into(),
            description: String::new(),
            category: BuildingCategory::Processing,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });
        reg.insert_recipe(RecipeDef {
            id: "refine_alloy".into(),
            name: "Refine Alloy".into(),
            building: "refinery".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "alloy".into(),
                quantity: 5.0,
            }],
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: false,
        });
        reg.insert_recipe(RecipeDef {
            id: "refine_gadget".into(),
            name: "Refine Gadget".into(),
            building: "refinery".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "gadget".into(),
                quantity: 3.0,
            }],
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: false,
        });
        reg
    }

    #[test]
    fn recipe_selection_defaults_to_first_recipe_when_unset() {
        let reg = make_registry_with_two_recipes();
        let mut pool = ColonyPool::new();
        let placed = buildings(&["refinery"]);

        process_production_scaled(
            &mut pool,
            &placed,
            10.0,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        assert!((pool.amount("alloy") - 5.0).abs() < 1e-9);
        assert_eq!(pool.amount("gadget"), 0.0);
    }

    #[test]
    fn recipe_selection_honors_active_recipe() {
        let reg = make_registry_with_two_recipes();
        let mut pool = ColonyPool::new();
        let placed = buildings(&["refinery"]);
        let mut active = std::collections::HashMap::new();
        active.insert("refinery".to_string(), "refine_gadget".to_string());

        process_production_scaled(
            &mut pool,
            &placed,
            10.0,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &active,
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        assert_eq!(pool.amount("alloy"), 0.0);
        assert!((pool.amount("gadget") - 3.0).abs() < 1e-9);
    }

    #[test]
    fn recipe_selection_falls_back_when_active_recipe_belongs_to_another_building() {
        let reg = make_registry_with_two_recipes();
        let mut pool = ColonyPool::new();
        let placed = buildings(&["refinery"]);
        let mut active = std::collections::HashMap::new();
        // Points at a recipe id that doesn't belong to "refinery" — must fall
        // back to the first authored recipe rather than silently no-op.
        active.insert("refinery".to_string(), "mine_ore".to_string());

        process_production_scaled(
            &mut pool,
            &placed,
            10.0,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &active,
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        assert!((pool.amount("alloy") - 5.0).abs() < 1e-9);
    }

    // ── Deposit gating (issue #239) ────────────────────────────────────────────

    fn make_registry_with_vein_mine() -> ContentRegistry {
        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "structural_mine".into(),
            name: "Structural Mine".into(),
            description: String::new(),
            category: BuildingCategory::Extraction,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });
        reg.insert_recipe(RecipeDef {
            id: "mine_structural_ore".into(),
            name: "Mine Structural Ore".into(),
            building: "structural_mine".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "structural_ore".into(),
                quantity: 10.0,
            }],
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: false,
        });
        // Non-deposit-gated recipe (not in VEIN_COMMODITIES) — a control to
        // prove gating is scoped to deposit-tracked commodities only.
        reg.insert_building(BuildingDef {
            id: "water_well".into(),
            name: "Water Well".into(),
            description: String::new(),
            category: BuildingCategory::Extraction,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });
        reg.insert_recipe(RecipeDef {
            id: "pump_water".into(),
            name: "Pump Water".into(),
            building: "water_well".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "water".into(),
                quantity: 10.0,
            }],
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: false,
        });
        reg
    }

    #[test]
    fn deposit_gating_is_inert_with_empty_richness_map() {
        // Empty map = colony has no spatial placement to check against
        // (e.g. founded via the bare Command::FoundColony path) — gating
        // must not apply at all, matching pre-#239 behaviour.
        let reg = make_registry_with_vein_mine();
        let mut pool = ColonyPool::new();
        let placed = buildings(&["structural_mine"]);

        let outcome = process_production_scaled(
            &mut pool,
            &placed,
            10.0,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        let mine = &outcome.building_results[0];
        assert!((mine.scale - 1.0).abs() < 1e-9);
        assert!(mine.shortfalls.is_empty());
        assert!((pool.amount("structural_ore") - 10.0).abs() < 1e-9);
    }

    #[test]
    fn deposit_gating_zeroes_output_with_no_matching_deposit() {
        // Non-empty richness map (colony IS spatially placed) but no entry
        // for structural_ore — no matching deposit at all.
        let reg = make_registry_with_vein_mine();
        let mut pool = ColonyPool::new();
        let placed = buildings(&["structural_mine"]);
        let mut deposits = std::collections::HashMap::new();
        deposits.insert("conductive_ore".to_string(), 0.8_f32); // unrelated commodity

        let outcome = process_production_scaled(
            &mut pool,
            &placed,
            10.0,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            Some(&deposits),
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        let mine = &outcome.building_results[0];
        assert!(
            mine.scale.abs() < 1e-9,
            "expected scale 0.0, got {}",
            mine.scale
        );
        assert!(mine.shortfalls.iter().any(|s| matches!(
            &s.reason,
            ShortfallReason::DepositShort { commodity_id } if commodity_id == "structural_ore"
        )));
        assert!(pool.amount("structural_ore").abs() < 1e-9);
    }

    #[test]
    fn deposit_gating_scales_output_by_richness_when_present() {
        let reg = make_registry_with_vein_mine();
        let mut pool = ColonyPool::new();
        let placed = buildings(&["structural_mine"]);
        let mut deposits = std::collections::HashMap::new();
        deposits.insert("structural_ore".to_string(), 0.4_f32);

        let outcome = process_production_scaled(
            &mut pool,
            &placed,
            10.0,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            Some(&deposits),
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        let mine = &outcome.building_results[0];
        // ratio = 0.5 + 0.4 * 0.5 = 0.7 (small epsilon: richness is stored
        // as f32 and widened to f64, so this isn't bit-exact).
        assert!(
            (mine.scale - 0.7).abs() < 1e-6,
            "expected scale ~0.7, got {}",
            mine.scale
        );
        assert!(mine.shortfalls.iter().any(|s| matches!(
            &s.reason,
            ShortfallReason::DepositShort { commodity_id } if commodity_id == "structural_ore"
        )));
        assert!((pool.amount("structural_ore") - 7.0).abs() < 1e-5);
    }

    #[test]
    fn deposit_gating_allows_full_output_at_full_richness() {
        let reg = make_registry_with_vein_mine();
        let mut pool = ColonyPool::new();
        let placed = buildings(&["structural_mine"]);
        let mut deposits = std::collections::HashMap::new();
        deposits.insert("structural_ore".to_string(), 1.0_f32);

        let outcome = process_production_scaled(
            &mut pool,
            &placed,
            10.0,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            Some(&deposits),
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        let mine = &outcome.building_results[0];
        assert!((mine.scale - 1.0).abs() < 1e-9);
        assert!(mine.shortfalls.is_empty());
    }

    #[test]
    fn deposit_gating_does_not_apply_to_non_vein_commodities() {
        // A colony IS spatially placed (non-empty map) but the water_well's
        // output ("water") isn't in VEIN_COMMODITIES, so it must run at full
        // scale regardless of what's (or isn't) in the deposits map.
        let reg = make_registry_with_vein_mine();
        let mut pool = ColonyPool::new();
        let placed = buildings(&["water_well"]);
        let mut deposits = std::collections::HashMap::new();
        deposits.insert("structural_ore".to_string(), 0.9_f32); // unrelated to water

        let outcome = process_production_scaled(
            &mut pool,
            &placed,
            10.0,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            Some(&deposits),
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        let well = &outcome.building_results[0];
        assert!((well.scale - 1.0).abs() < 1e-9);
        assert!(well.shortfalls.is_empty());
        assert!((pool.amount("water") - 10.0).abs() < 1e-9);
    }

    // ── Concurrent (multi-function) recipes ──────────────────────────────────

    /// A `colony_hq`-style building with two always-on `concurrent` recipes
    /// (power generation + water purification) and no pick-one recipe at
    /// all — the minimal "multi-function starter building" shape.
    fn make_registry_with_concurrent_recipes() -> ContentRegistry {
        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "colony_hq".into(),
            name: "Colony HQ".into(),
            description: String::new(),
            category: BuildingCategory::Services,
            construction_cost: vec![],
            power_delta: -20.0, // net generator once its own recipe draw is netted
            worker_slots: 2,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });
        reg.insert_recipe(RecipeDef {
            id: "hq_generate_power".into(),
            name: "Generate Power".into(),
            building: "colony_hq".into(),
            inputs: vec![],
            outputs: vec![],
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: true,
        });
        reg.insert_recipe(RecipeDef {
            id: "hq_purify_water".into(),
            name: "Purify Water".into(),
            building: "colony_hq".into(),
            inputs: vec![Ingredient {
                id: "raw_water".into(),
                quantity: 2.0,
            }],
            outputs: vec![Ingredient {
                id: "water".into(),
                quantity: 2.0,
            }],
            cycle_sols: 1,
            power_draw: 5.0,
            concurrent: true,
        });
        reg
    }

    #[test]
    fn concurrent_recipes_all_run_simultaneously_with_no_active_recipe_selection() {
        let reg = make_registry_with_concurrent_recipes();
        let mut pool = ColonyPool::new();
        pool.deposit("raw_water", 10.0);
        let placed = buildings(&["colony_hq"]);

        let outcome = process_production(&mut pool, &placed, 10.0_f32, &reg);

        let hq = outcome
            .building_results
            .iter()
            .find(|r| r.building_type == "colony_hq")
            .unwrap();
        // No pick-one recipe exists for this building — recipe_id is empty —
        // but both concurrent recipes ran.
        assert_eq!(hq.recipe_id, "");
        let mut ids = hq.concurrent_recipe_ids.clone();
        ids.sort();
        assert_eq!(ids, vec!["hq_generate_power", "hq_purify_water"]);
        assert!(hq.is_full_production(), "shortfalls: {:?}", hq.shortfalls);

        // Purify-water's input/output actually applied.
        assert!((pool.amount("raw_water") - 8.0).abs() < 1e-9);
        assert!((pool.amount("water") - 2.0).abs() < 1e-9);
    }

    #[test]
    fn concurrent_recipe_power_draw_is_summed_into_grid_demand() {
        let reg = make_registry_with_concurrent_recipes();
        let mut pool = ColonyPool::new();
        pool.deposit("raw_water", 10.0);
        let placed = buildings(&["colony_hq"]);

        let outcome = process_production(&mut pool, &placed, 10.0_f32, &reg);

        // colony_hq's own power_delta is -20 (generator); hq_purify_water
        // draws 5 kW; hq_generate_power draws 0. Net capacity 20, demand 5.
        assert!((outcome.power_grid.capacity - 20.0).abs() < 1e-9);
        assert!((outcome.power_grid.demand - 5.0).abs() < 1e-9);
    }

    #[test]
    fn concurrent_recipe_input_shortfall_scales_the_whole_building_down() {
        // Only 1 unit of raw_water available; hq_purify_water needs 2 ->
        // input_ratio = 0.5. Since there's no other recipe/demand on this
        // building, the whole instance's shared scale drops to 0.5.
        let reg = make_registry_with_concurrent_recipes();
        let mut pool = ColonyPool::new();
        pool.deposit("raw_water", 1.0);
        let placed = buildings(&["colony_hq"]);

        let outcome = process_production(&mut pool, &placed, 10.0_f32, &reg);

        let hq = outcome
            .building_results
            .iter()
            .find(|r| r.building_type == "colony_hq")
            .unwrap();
        assert!((hq.scale - 0.5).abs() < 1e-9, "scale was {}", hq.scale);
        assert!(hq
            .shortfalls
            .iter()
            .any(|s| matches!(s.reason, ShortfallReason::InputShort { .. })));
        assert!((pool.amount("water") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn concurrent_recipe_never_becomes_the_pick_one_default_or_selectable() {
        // A building with ONE concurrent recipe and no pick-one recipes at
        // all must resolve recipe_for_building -> None (it must not also be
        // picked as "the" default, which would double-run it), while still
        // running via concurrent_recipes_for_building.
        let reg = make_registry_with_concurrent_recipes();
        assert!(
            recipe_for_building("colony_hq", &std::collections::HashMap::new(), &reg).is_none()
        );
        let concurrent = concurrent_recipes_for_building("colony_hq", &reg);
        assert_eq!(concurrent.len(), 2);

        // Even an explicit (mistaken) active_recipes selection naming a
        // concurrent recipe must not resolve it as the pick-one recipe.
        let mut active = std::collections::HashMap::new();
        active.insert("colony_hq".to_string(), "hq_purify_water".to_string());
        assert!(recipe_for_building("colony_hq", &active, &reg).is_none());
    }

    #[test]
    fn pick_one_recipe_and_concurrent_recipes_coexist_and_share_one_scale() {
        // A building with both a pick-one recipe (selected via
        // active_recipes) and a concurrent recipe: both run together, both
        // count toward the same shared scale/shortfalls.
        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "hybrid_plant".into(),
            name: "Hybrid Plant".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 1,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });
        reg.insert_recipe(RecipeDef {
            id: "hybrid_alt_a".into(),
            name: "Alt A".into(),
            building: "hybrid_plant".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "widget_a".into(),
                quantity: 1.0,
            }],
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: false,
        });
        reg.insert_recipe(RecipeDef {
            id: "hybrid_alt_b".into(),
            name: "Alt B".into(),
            building: "hybrid_plant".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "widget_b".into(),
                quantity: 1.0,
            }],
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: false,
        });
        reg.insert_recipe(RecipeDef {
            id: "hybrid_always_on".into(),
            name: "Always On".into(),
            building: "hybrid_plant".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "widget_c".into(),
                quantity: 1.0,
            }],
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: true,
        });

        let mut pool = ColonyPool::new();
        let placed = buildings(&["hybrid_plant"]);
        let mut active = std::collections::HashMap::new();
        active.insert("hybrid_plant".to_string(), "hybrid_alt_b".to_string());

        // Default (no active selection passed to process_production, which
        // hardcodes an empty active_recipes map) picks the first pick-one
        // alternative alphabetically: hybrid_alt_a.
        process_production(&mut pool, &placed, 10.0_f32, &reg);
        assert!((pool.amount("widget_a") - 1.0).abs() < 1e-9);
        assert!((pool.amount("widget_c") - 1.0).abs() < 1e-9);
        assert_eq!(pool.amount("widget_b"), 0.0);

        // Now explicitly select hybrid_alt_b via active_recipes.
        let mut pool2 = ColonyPool::new();
        process_production_scaled(
            &mut pool2,
            &placed,
            10.0,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &active,
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );
        assert!((pool2.amount("widget_b") - 1.0).abs() < 1e-9);
        assert!((pool2.amount("widget_c") - 1.0).abs() < 1e-9);
        assert_eq!(pool2.amount("widget_a"), 0.0);
    }

    // ── Building-level I/O summary (issue #272) ───────────────────────────

    /// Build a registry with a multi-function building whose recipes are *all*
    /// concurrent — the `colony_hq` shape — plus one with a pick-one choice.
    fn make_registry_for_io_summary() -> ContentRegistry {
        let mut reg = ContentRegistry::default();
        let building = |id: &str| BuildingDef {
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
        };
        reg.insert_building(building("hq"));
        reg.insert_building(building("refinery"));
        reg.insert_building(building("silo"));

        let recipe = |id: &str,
                      b: &str,
                      concurrent: bool,
                      inputs: Vec<(&str, f64)>,
                      outputs: Vec<(&str, f64)>| RecipeDef {
            id: id.into(),
            name: id.into(),
            building: b.into(),
            cycle_sols: 1,
            inputs: inputs
                .into_iter()
                .map(|(id, quantity)| Ingredient {
                    id: id.into(),
                    quantity,
                })
                .collect(),
            outputs: outputs
                .into_iter()
                .map(|(id, quantity)| Ingredient {
                    id: id.into(),
                    quantity,
                })
                .collect(),
            concurrent,
            power_draw: 0.0,
        };

        // Three always-on recipes, two of which produce the *same* commodity.
        reg.insert_recipe(recipe("hq_power", "hq", true, vec![], vec![("power", 6.0)]));
        reg.insert_recipe(recipe("hq_water", "hq", true, vec![], vec![("water", 4.0)]));
        reg.insert_recipe(recipe(
            "hq_trickle",
            "hq",
            true,
            vec![("water", 1.0)],
            vec![("power", 2.0)],
        ));

        // Pick-one alternatives plus one always-on recipe alongside them.
        reg.insert_recipe(recipe(
            "refine_a",
            "refinery",
            false,
            vec![("ore", 2.0)],
            vec![("metal", 1.0)],
        ));
        reg.insert_recipe(recipe(
            "refine_b",
            "refinery",
            false,
            vec![("ore", 4.0)],
            vec![("alloy", 1.0)],
        ));
        reg.insert_recipe(recipe(
            "refinery_vent",
            "refinery",
            true,
            vec![],
            vec![("waste_heat", 3.0)],
        ));
        reg
    }

    /// The motivating case: a building whose recipes are all concurrent has no
    /// pick-one recipe, so the old `recipe_id`-only view showed nothing at all.
    /// The summary must report every running recipe and the merged flows.
    #[test]
    fn io_summary_covers_an_all_concurrent_building() {
        let reg = make_registry_for_io_summary();
        let summary = building_io_summary("hq", &std::collections::HashMap::new(), &reg);

        assert_eq!(
            summary.recipe_ids,
            vec!["hq_power", "hq_trickle", "hq_water"],
            "all three always-on recipes run, in id order"
        );
        // power appears in two recipes: 6 + 2 = 8, merged rather than listed twice.
        assert_eq!(
            summary.outputs,
            vec![("power".to_string(), 8.0), ("water".to_string(), 4.0)]
        );
        assert_eq!(summary.inputs, vec![("water".to_string(), 1.0)]);
        assert!(!summary.is_empty());
    }

    /// A commodity that is both consumed and produced stays in both lists —
    /// netting it would conflate throughput with net change.
    #[test]
    fn io_summary_does_not_net_a_commodity_against_itself() {
        let reg = make_registry_for_io_summary();
        let summary = building_io_summary("hq", &std::collections::HashMap::new(), &reg);
        assert!(summary.inputs.iter().any(|(id, _)| id == "water"));
        assert!(summary.outputs.iter().any(|(id, _)| id == "water"));
    }

    /// The summary follows the player's recipe selection, and always includes
    /// the always-on recipes alongside it.
    #[test]
    fn io_summary_follows_the_selected_pick_one_recipe() {
        let reg = make_registry_for_io_summary();

        let default = building_io_summary("refinery", &std::collections::HashMap::new(), &reg);
        assert_eq!(
            default.recipe_ids,
            vec!["refine_a", "refinery_vent"],
            "the deterministic default plus the always-on one"
        );
        assert_eq!(default.inputs, vec![("ore".to_string(), 2.0)]);

        let mut active = std::collections::HashMap::new();
        active.insert("refinery".to_string(), "refine_b".to_string());
        let selected = building_io_summary("refinery", &active, &reg);
        assert_eq!(selected.recipe_ids, vec!["refine_b", "refinery_vent"]);
        assert_eq!(selected.inputs, vec![("ore".to_string(), 4.0)]);
        assert!(
            selected.outputs.iter().any(|(id, _)| id == "alloy"),
            "the selected recipe's output should show, not the default's"
        );
        assert!(
            selected.outputs.iter().any(|(id, _)| id == "waste_heat"),
            "the always-on recipe runs regardless of selection"
        );
    }

    /// A pure storage building has no recipes and therefore an empty summary —
    /// not a panic, and not a phantom row.
    #[test]
    fn io_summary_is_empty_for_a_building_with_no_recipes() {
        let reg = make_registry_for_io_summary();
        let summary = building_io_summary("silo", &std::collections::HashMap::new(), &reg);
        assert!(summary.recipe_ids.is_empty());
        assert!(summary.is_empty());
    }

    /// The summary must describe the set the production step actually runs. If
    /// these drift apart the panel starts lying, so pin them against each other.
    #[test]
    fn io_summary_matches_what_production_actually_ran() {
        let reg = make_registry_for_io_summary();
        let mut pool = ColonyPool::new();
        pool.deposit("water", 100.0);
        let outcome = process_production(&mut pool, &[("hq".to_string(), 1)], 100.0, &reg);

        let result = outcome
            .building_results
            .iter()
            .find(|r| r.building_type == "hq")
            .expect("hq should have produced");
        let summary = building_io_summary("hq", &std::collections::HashMap::new(), &reg);

        let mut ran: Vec<String> = result.concurrent_recipe_ids.clone();
        if !result.recipe_id.is_empty() {
            ran.push(result.recipe_id.clone());
        }
        ran.sort();
        let mut summarised = summary.recipe_ids.clone();
        summarised.sort();
        assert_eq!(
            ran, summarised,
            "the summary and the production step must agree on which recipes run"
        );
    }
}
