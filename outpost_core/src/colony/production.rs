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

use uuid::Uuid;

use crate::content::types::Ingredient;
use crate::content::{BuildingCategory, ContentRegistry, RecipeDef};

use super::building::PlacedBuilding;
use super::labour::{LabourCandidate, LabourPlan};

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
    /// How much more of the binding commodity full output would have needed
    /// (issues #303, #308).
    ///
    /// `0.0` for shortfalls with no commodity to quantify — a power brownout or
    /// a labour shortage. `#[serde(default)]` so pre-#308 saves and stored
    /// results load with `0.0` rather than failing; this is per-sol derived data
    /// that the next advance regenerates.
    ///
    /// Carried because [`Self::effective_scale`] on its own says "30 %" without
    /// saying whether that is two units short or two hundred.
    #[serde(default)]
    pub deficit: f64,
}

/// Category of shortfall limiting a building's production.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum ShortfallReason {
    /// One or more input commodities were insufficient, and **nothing in this
    /// colony produces the binding one**.
    ///
    /// This is the "you have no supply line" case: build a producer, open a trade
    /// route, or stop running this recipe. Contrast
    /// [`ShortfallReason::AwaitingUpstream`].
    InputShort {
        /// The commodity id that was the tightest constraint.
        commodity_id: String,
    },
    /// The binding input **is** produced by a building in this colony, so the
    /// shortage is a pipeline that has not filled rather than a missing supply
    /// line (issue #308).
    ///
    /// Production reads a start-of-turn snapshot on purpose — that is what keeps
    /// the pass order-independent — so a chain costs one sol per stage to fill
    /// before it flows at full rate. During that fill every downstream building
    /// is genuinely short, but telling the player "input short" invites them to
    /// go build a second mine they do not need.
    ///
    /// Follow this up the chain to find the real cause: each level that has a
    /// local producer reports `AwaitingUpstream`, and the level that reports
    /// something else — [`ShortfallReason::InputShort`],
    /// [`ShortfallReason::DepositShort`], [`ShortfallReason::LaborShort`] — is
    /// where the problem actually is. If every level reports `AwaitingUpstream`,
    /// the pipeline is simply still filling and will resolve on its own.
    AwaitingUpstream {
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
    /// colony's site/body (issue #239).
    ///
    /// `effective_scale` on the enclosing [`ProductionShortfall`] says which case
    /// it is. Since #317 a total absence is no longer `0.0` but
    /// [`TRACE_DEPOSIT_RATIO`] — scraping bare ground — so the readable bands are:
    ///
    /// - `== TRACE_DEPOSIT_RATIO`: no deposit here at all; prospect, or accept the
    ///   trickle.
    /// - `0.5..1.0`: a real but sub-maximal deposit; richer ground exists.
    DepositShort {
        /// The deposit-gated commodity id that was the tightest constraint.
        commodity_id: String,
    },
}

/// Outcome of one building's production attempt this turn.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildingProductionResult {
    /// The placed instance this result belongs to (issue #307).
    ///
    /// Per-building labour means two buildings of the same type can now run at
    /// different scales, so a type key no longer identifies a result.
    /// `#[serde(default)]` — a pre-#307 save loads with a nil id, which is
    /// harmless: this is per-sol derived data, regenerated on the next advance.
    #[serde(default)]
    pub building_id: Uuid,
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
    /// Scale factor for the building as a whole, in `[0.0, 1.0]`.
    ///
    /// Since #272 lines throttle independently, so this is the **worst** line's
    /// scale — a summary that still answers "is anything wrong here?" correctly
    /// and keeps pre-#272 consumers working. Read [`Self::line_results`] for what
    /// each line actually achieved.
    pub scale: f64,
    /// Every line's shortfalls, flattened. See [`Self::line_results`] for which
    /// line each belongs to.
    pub shortfalls: Vec<ProductionShortfall>,
    /// Per-line outcome (issue #272), one entry per running production line.
    ///
    /// Purely additive: `#[serde(default)]` so pre-#272 saves load, and the
    /// fields above keep their old meanings for consumers that don't know about
    /// lines.
    #[serde(default)]
    pub line_results: Vec<LineProductionResult>,
}

/// What one production line achieved this turn (issue #272).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LineProductionResult {
    /// Authored line name; `None` is the building's default line.
    pub line: Option<String>,
    /// `true` for a line derived from a `concurrent` recipe.
    pub always_on: bool,
    /// Recipe that ran on this line.
    pub recipe_id: String,
    /// This line's own scale, in `[0.0, 1.0]`.
    pub scale: f64,
    /// Shortfalls that held this line back.
    pub shortfalls: Vec<ProductionShortfall>,
    /// `(commodity_id, amount)` actually deposited per output this sol, after
    /// every yield multiplier (productivity/category/tech) has been applied
    /// (issue #317). Purely additive: `#[serde(default)]` so pre-#317 saves
    /// load with an empty vec.
    #[serde(default)]
    pub outputs_deposited: Vec<(String, f64)>,
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
    ///
    /// Since #307 this is the sum of the **gated** per-building demands, so a
    /// building that couldn't run at all this sol (no inputs, no power) doesn't
    /// inflate it. It therefore reports jobs the colony could actually fill,
    /// which is what "understaffed" should be measured against.
    pub labor_demanded: f32,
    /// How the workforce was actually distributed across buildings (issue #307).
    pub labour: LabourPlan,
}

// ─── Internal helpers for two-pass production resolution ─────────────────────

/// Holds a building's pre-computed scale before any pool mutations occur.
struct PendingProduction<'a> {
    /// Placed instance this belongs to — the key labour is allocated against.
    building_id: Uuid,
    building_type: &'a str,
    /// One entry per production line, each with its **own** scale (issue #272).
    lines: Vec<PendingLine<'a>>,
    building_category: BuildingCategory,
    maintenance: &'a [Ingredient],
    /// Effective per-sol maintenance multiplier
    /// (`MaintenanceConsumption` scalar; `0.0` when disabled).
    maintenance_multiplier: f64,
    /// A maintenance-only building's own affordability ratio, which needs no
    /// labour input. `None` for a building with lines, whose upkeep scale is the
    /// busiest line's and so isn't known until labour has been folded in.
    maintenance_only_scale: Option<f64>,
    /// Scale upkeep is charged at, resolved in Pass L.
    maintenance_scale: f64,
    /// Whether this building consumes labour at all (issue #307). A pure
    /// maintenance-only building doesn't, and is never reported labour-short.
    applies_labour: bool,
    /// Workers wanted this sol, already gated on being able to run.
    labour_demand: u32,
    priority: u8,
    labour_lock: Option<u32>,
}

/// One line's resolved recipe and its own scale for this turn (issue #272).
struct PendingLine<'a> {
    line: Option<String>,
    always_on: bool,
    recipe: &'a RecipeDef,
    scale: f64,
    shortfalls: Vec<ProductionShortfall>,
    /// `(commodity_id, amount)` actually deposited per output this sol, filled
    /// in Pass B after every yield multiplier has been applied (issue #317) —
    /// the true figure, not `recipe.outputs[].quantity * scale`, which skips
    /// the productivity/category/tech multipliers applied alongside it.
    outputs_deposited: Vec<(String, f64)>,
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

// ─── Production input ─────────────────────────────────────────────────────────

/// One placed building as the production pipeline sees it.
///
/// Production used to take bare `(building_type, slot_cost)` pairs. Per-building
/// labour (#307) needs **instance identity** — two mines of the same type can be
/// staffed differently now, so a type key can no longer stand in for a building.
/// Priority and lock ride along because production is where labour is allocated:
/// demand has to be gated on whether each building can actually run, which isn't
/// known until input, power, and deposit availability have been resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionInput {
    /// Stable identifier of the placed instance.
    pub id: Uuid,
    /// Content-pack key identifying the building type.
    pub building_type: String,
    /// Build slots consumed (carried through for power-grid and slot accounting).
    pub slot_cost: u32,
    /// Staffing priority: `1` is staffed first.
    pub priority: u8,
    /// Player-pinned labour allocation, if any.
    pub labour_lock: Option<u32>,
}

impl ProductionInput {
    /// Build an input from a placed building, carrying its priority and lock.
    #[must_use]
    pub fn from_placed(building: &PlacedBuilding) -> Self {
        Self {
            id: building.id,
            building_type: building.building_type.clone(),
            slot_cost: building.slot_cost,
            priority: building.priority,
            labour_lock: building.labour_lock,
        }
    }

    /// Build inputs from bare `(building_type, slot_cost)` pairs, at the default
    /// priority with no locks.
    ///
    /// For callers with no per-instance staffing state to express — outposts run
    /// a fixed skeleton crew (see [`crate::outpost::OUTPOST_BASE_LABOR`]), and
    /// most tests only care about a commodity chain. Identifiers are synthesised
    /// **from the slice index** rather than randomly, so the allocator's
    /// `(priority, building_type, id)` tiebreak stays reproducible — a random id
    /// here would make equal-priority allocation vary between runs.
    #[must_use]
    pub fn from_types(buildings: &[(String, u32)]) -> Vec<Self> {
        buildings
            .iter()
            .enumerate()
            .map(|(index, (building_type, slot_cost))| Self {
                id: Uuid::from_u128(index as u128),
                building_type: building_type.clone(),
                slot_cost: *slot_cost,
                priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
                labour_lock: None,
            })
            .collect()
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
    buildings: &[ProductionInput],
    labor_available: f32,
    registry: &ContentRegistry,
) -> ProductionStepOutcome {
    process_production_scaled(
        stores,
        buildings,
        labor_available,
        registry,
        1.0,
        0.0, // power_import (issue #383)
        1.0,
        true,
        1.0,
        &std::collections::HashMap::new(),
        &[],
        None,
        &crate::modifier::ModifierAccumulator::new(),
        &crate::modifier::DifficultyScalar::new(),
        // No site on the plain path — callers with one use
        // `process_production_scaled` directly (issue #411).
        None,
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
/// `site_multipliers` (issue #411) maps building type → output multiplier
/// derived from where the colony stands, via
/// [`crate::site::SiteContext::output_multiplier`]. Applied to both recipe
/// outputs and the building's grid-capacity contribution. `None`, or a type
/// absent from the map, means `1.0` — a site that cannot be read leaves a
/// building performing exactly as it did before the mechanism existed.
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
    buildings: &[ProductionInput],
    labor_available: f32,
    registry: &ContentRegistry,
    power_scalar: f32,
    power_import: f64,
    maintenance_scalar: f32,
    maintenance_enabled: bool,
    productivity_multiplier: f32,
    active_recipes: &std::collections::HashMap<String, String>,
    category_modifiers: &[crate::system::BodyModifier],
    deposit_richness: Option<&std::collections::HashMap<String, f32>>,
    modifier_accumulator: &crate::modifier::ModifierAccumulator,
    difficulty_scalar: &crate::modifier::DifficultyScalar,
    site_multipliers: Option<&std::collections::HashMap<String, f64>>,
) -> ProductionStepOutcome {
    // ── Step 1: build power grid ─────────────────────────────────────────────
    // `power_import` folds in this sol's net cross-colony powerline transfer
    // (issue #383) — positive for a colony drawing a surplus neighbor's power
    // net of transmission loss, negative for a colony exporting its own
    // surplus out. Callers that haven't resolved transfers (e.g. outposts,
    // which have no edges) pass `0.0`.
    let mut power_grid = compute_power_grid_scaled(
        buildings,
        registry,
        power_scalar,
        active_recipes,
        site_multipliers,
    );
    power_grid.capacity = (power_grid.capacity + power_import).max(0.0);
    let brownout_ratio = power_grid.supply_ratio();

    // ── Step 2: three-pass resolution ─────────────────────────────────────────
    //
    // Pass A: compute every line's scale from input, power, and deposit
    //         availability, against the *start-of-turn* pool state so that mines
    //         and other producers don't inflate inputs for downstream buildings
    //         in the same turn.
    // Pass L: allocate labour across the buildings, then fold each building's
    //         staffing ratio into its lines' scales.
    // Pass B: apply all scaled changes to the stores.
    //
    // Labour sits **between** the other constraints and the application step,
    // rather than being computed up front with them, because demand depends on
    // what Pass A found (#307): a building with no ore left to dig or no power to
    // run on offers no jobs this sol, and its workers should be free to go
    // somewhere they can accomplish something. Pass A therefore can't already
    // know its labour ratio, and Pass L can't run before it.
    //
    // The A/B split avoids order-dependency and matches the C# "snapshot then
    // apply" behaviour described in the behavioural spec.

    let mut pending: Vec<PendingProduction<'_>> = Vec::new();
    let maintenance_multiplier = if maintenance_enabled {
        f64::from(maintenance_scalar.max(0.0))
    } else {
        0.0
    };

    // Scarce inputs are claimed in **priority order** (issue #308), the same
    // `(priority, building_type, id)` order the labour allocator uses — so one
    // lever steers both, and a building the player ranked first isn't starved of
    // feedstock by a building they ranked last.
    //
    // Without this, every building was judged against the whole pool
    // independently: two buildings each needing the last 10 water both ran at
    // full rate, the second producing a full batch from an empty pool.
    // Consequence worth knowing: `building_results` — and therefore the
    // `ProductionShortfall` events built from it — now come out in priority order
    // rather than placement order. That makes the sequence independent of the
    // order buildings happened to be built in, but it does change what a player
    // watching a shortfall log sees. Nothing reads these positionally:
    // `last_production_by_building` is keyed by instance id, and `ui::BuildingRow`
    // iterates the colony's own building list and looks results up by that key.
    let mut order: Vec<&ProductionInput> = buildings.iter().collect();
    order.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.building_type.cmp(&b.building_type))
            .then_with(|| a.id.cmp(&b.id))
    });

    // Recipe lines resolved once per **distinct building type**, not once per
    // placed instance.
    //
    // `lines_for_building` scans every recipe in the registry and builds a
    // `BTreeMap` to group them, and this is the per-turn hot path — so calling it
    // for each of a colony's twelve mines twelve times over was already wasteful
    // before #308 needed the same answer a second time for `locally_produced`
    // below. Memoising by type collapses both into one pass per type.
    let mut lines_by_type: std::collections::HashMap<&str, Vec<RecipeLine<'_>>> =
        std::collections::HashMap::new();
    for input in buildings {
        if !lines_by_type.contains_key(input.building_type.as_str()) {
            lines_by_type.insert(
                input.building_type.as_str(),
                lines_for_building(&input.building_type, active_recipes, registry),
            );
        }
    }

    // Every commodity some building here is set up to produce (issue #308).
    //
    // Membership is what separates "this pipeline hasn't filled yet" from "you
    // have no supply line" in the shortfall readout. Deliberately based on what
    // the colony is *configured* to make, not on what it managed to make this
    // sol: a mine that itself sat idle is still the answer to "where does ore
    // come from here", and reporting `InputShort` at the smelter would send the
    // player looking in the wrong place.
    let locally_produced: std::collections::HashSet<&str> = lines_by_type
        .values()
        .flatten()
        .flat_map(|line| line.selected.outputs.iter().map(|out| out.id.as_str()))
        .collect();

    // commodity id → quantity already claimed by a better-priority building.
    //
    // Pre-seeded with the player's commodity reserves (issue #308), which makes a
    // reserve simply the first claim on the stockpile — one nobody outbids,
    // because nothing ever removes an entry from this map. That is why a reserve
    // needs no separate code path: it withholds stock from the whole production
    // pass, recipe inputs and maintenance alike, while colonist needs — resolved
    // in step 2, *before* production even runs — see the untouched pool and can
    // still eat it. Needs going first is what makes the reserve safe by
    // construction: colonists are fed before industry is considered at all, so
    // withholding stock can never starve them however large the reserve.
    let mut reserved: std::collections::HashMap<String, f64> = stores.reserve_claims();

    for input in order {
        let building_type = &input.building_type;
        let Some(bdef) = registry.building(building_type) else {
            continue; // unknown building type — skip
        };
        // Each line throttles on its own inputs (issue #272), so a starved
        // smelting line no longer drags the machining line beside it down.
        // Resolved once per type above rather than recomputed per instance.
        let building_lines = lines_by_type
            .get(building_type.as_str())
            .map_or(&[][..], Vec::as_slice);
        let has_any_recipe = !building_lines.is_empty();
        let has_maintenance = maintenance_enabled && !bdef.maintenance.is_empty();

        // Buildings with neither a recipe nor an active maintenance list stay
        // out of the production pass entirely (unchanged pre-#180 behaviour).
        if !has_any_recipe && !has_maintenance {
            continue;
        }

        let maintenance_slice = if has_maintenance {
            bdef.maintenance.as_slice()
        } else {
            &[][..]
        };

        // Power applies only to recipe-running buildings: a pure
        // maintenance-only building doesn't drive brownouts. Power is computed
        // colony-wide, so it applies to every line equally.
        //
        // `applies_labour` marks buildings that consume labour at all — the same
        // recipe-running set. A maintenance-only building is never labour-short.
        let (power_ratio, applies_labour) = if has_any_recipe {
            let pr = if bdef.category == BuildingCategory::Power {
                1.0
            } else {
                brownout_ratio
            };
            (pr, true)
        } else {
            (1.0, false)
        };

        let mut pending_lines: Vec<PendingLine<'_>> = Vec::new();

        // A maintenance-only building still needs an entry so its upkeep is
        // charged; it just has no lines to run.
        for line in building_lines {
            // Per-line demand: this line's own recipe inputs, pooled with the
            // building's maintenance. Maintenance is pooled into every line
            // rather than split between them because it is a building-level
            // upkeep that competes with whatever each line is trying to draw —
            // and for a building with a single line (every shipped building
            // today) that makes this identical to the pre-#272 calculation.
            // Upkeep is still *withdrawn* only once, below.
            let afford = compute_effective_input_ratio(
                stores,
                &reserved,
                Some(line.selected),
                &[],
                maintenance_slice,
                maintenance_multiplier,
            );
            let (input_ratio, tight_commodity, tight_is_maintenance) = (
                afford.ratio,
                afford.tight.clone(),
                afford.tight_is_maintenance,
            );

            // Deposit gating (issue #239) — only applies to deposit-gated
            // recipes; inert (ratio 1.0) for everything else.
            let (deposit_ratio, deposit_tight) =
                compute_deposit_ratio(Some(line.selected), &[], deposit_richness);

            // Labour is deliberately absent here — it is folded in by Pass L
            // once demand has been gated on this very scale (#307).
            let scale = input_ratio.min(power_ratio).min(deposit_ratio).max(0.0);

            // Record shortfalls. Maintenance-only tight constraints report as
            // `MaintenanceShort`; shared input+maintenance constraints stay as
            // `InputShort` for backwards compatibility with pre-#180 events.
            let mut shortfalls: Vec<ProductionShortfall> = Vec::new();
            if input_ratio < 1.0 - 1e-9 {
                let commodity_id = tight_commodity.unwrap_or_default();
                let reason = if tight_is_maintenance {
                    ShortfallReason::MaintenanceShort { commodity_id }
                } else if locally_produced.contains(commodity_id.as_str()) {
                    // Something here makes this — the chain is filling or the
                    // real fault is further upstream (#308).
                    ShortfallReason::AwaitingUpstream { commodity_id }
                } else {
                    ShortfallReason::InputShort { commodity_id }
                };
                shortfalls.push(ProductionShortfall {
                    reason,
                    effective_scale: input_ratio,
                    deficit: afford.tight_deficit,
                });
            }
            if power_ratio < 1.0 - 1e-9 {
                shortfalls.push(ProductionShortfall {
                    reason: ShortfallReason::PowerBrownout,
                    effective_scale: power_ratio,
                    // Power is a ratio across the whole grid, not a per-building
                    // commodity draw, so there is no single deficit to name here.
                    deficit: 0.0,
                });
            }
            if deposit_ratio < 1.0 - 1e-9 {
                shortfalls.push(ProductionShortfall {
                    reason: ShortfallReason::DepositShort {
                        commodity_id: deposit_tight.unwrap_or_default(),
                    },
                    effective_scale: deposit_ratio,
                    // Deposit richness scales yield; nothing was withheld from a
                    // stockpile, so a deficit quantity would be meaningless.
                    deficit: 0.0,
                });
            }
            // Claim what this line will actually draw, so the next building in
            // priority order sees a pool that reflects it.
            //
            // Reserved at the **pre-labour** scale: labour is allocated later
            // (Pass L) and can only reduce it, so this can over-claim for a
            // building that later turns out to be understaffed. Erring toward the
            // better-priority building is the intended bias, and Pass B still
            // withdraws only the final amount, so nothing is over-drawn — the
            // surplus is simply not re-offered to anyone else this sol.
            for (id, qty) in &afford.recipe_demands {
                reserve(&mut reserved, id, qty * scale);
            }

            pending_lines.push(PendingLine {
                line: line.line.clone(),
                always_on: line.always_on,
                recipe: line.selected,
                scale,
                shortfalls,
                outputs_deposited: Vec::new(),
            });
        }

        // A maintenance-only building has no lines, so its upkeep scale is just
        // its own affordability ratio and doesn't depend on labour. For everything
        // else the scale is the busiest line's, which isn't known until Pass L has
        // folded labour in — so it's computed there.
        let maintenance_only_scale = if has_any_recipe {
            None
        } else {
            let afford = compute_effective_input_ratio(
                stores,
                &reserved,
                None,
                &[],
                maintenance_slice,
                maintenance_multiplier,
            );
            Some(afford.ratio.max(0.0))
        };

        // Upkeep is charged once per building, so reserve it once — at the
        // busiest pre-labour line's scale, matching how it is charged in Pass B.
        let upkeep_scale = maintenance_only_scale.unwrap_or_else(|| {
            pending_lines
                .iter()
                .map(|l| l.scale)
                .fold(0.0_f64, f64::max)
        });
        if upkeep_scale > 1e-9 {
            for (id, qty) in maintenance_draws(maintenance_slice, maintenance_multiplier) {
                reserve(&mut reserved, &id, qty * upkeep_scale);
            }
        }

        // What this building would want if it can run at all. A building whose
        // lines are all stalled — no inputs, no power, no deposit — offers no
        // jobs, so its workers are freed for buildings that can use them (#307).
        let can_run = pending_lines.iter().any(|l| l.scale > 1e-9);
        let labour_demand = if applies_labour && can_run {
            bdef.worker_slots
        } else {
            0
        };

        pending.push(PendingProduction {
            building_id: input.id,
            building_type,
            lines: pending_lines,
            building_category: bdef.category.clone(),
            maintenance: maintenance_slice,
            maintenance_multiplier,
            maintenance_only_scale,
            maintenance_scale: 0.0,
            applies_labour,
            labour_demand,
            priority: input.priority,
            labour_lock: input.labour_lock,
        });
    }

    // ── Pass L: allocate labour, then fold it into the scales ─────────────────
    //
    // This is what replaces the old colony-wide ratio (#307). Before, every
    // building was throttled by `available / demanded` alike, so a shortage hurt
    // the greenhouse exactly as much as the ore mine. Now the workforce is handed
    // out in priority order and a building either gets its staff or doesn't.
    let labour_candidates: Vec<LabourCandidate> = pending
        .iter()
        .map(|p| LabourCandidate {
            id: p.building_id,
            building_type: p.building_type.to_owned(),
            priority: p.priority,
            labour_lock: p.labour_lock,
            demand: p.labour_demand,
        })
        .collect();
    let labour_plan = super::labour::allocate_from(&labour_candidates, workforce(labor_available));

    // Summed as f64 then narrowed: worker counts are small enough that f32 is
    // exact in practice, but the sum goes through f64 so the total can't drift.
    let labor_demanded = labour_candidates
        .iter()
        .map(|c| f64::from(c.demand))
        .sum::<f64>();
    #[allow(clippy::cast_possible_truncation)]
    let labor_demanded = labor_demanded as f32;

    for p in &mut pending {
        let labour_ratio = if p.applies_labour {
            labour_plan
                .for_building(p.building_id)
                .map_or(1.0, super::labour::LabourAllocation::ratio)
        } else {
            1.0
        };

        if labour_ratio < 1.0 - 1e-9 {
            for line in &mut p.lines {
                line.shortfalls.push(ProductionShortfall {
                    reason: ShortfallReason::LaborShort,
                    effective_scale: labour_ratio,
                    // Workers, not a commodity draw — no stockpile deficit to name.
                    deficit: 0.0,
                });
                line.scale = line.scale.min(labour_ratio).max(0.0);
            }
        }

        // Upkeep is charged once for the building, at the busiest line's scale —
        // a building that produced nothing pays nothing, matching pre-#272
        // behaviour where a zero scale skipped maintenance entirely.
        p.maintenance_scale = p
            .maintenance_only_scale
            .unwrap_or_else(|| p.lines.iter().map(|l| l.scale).fold(0.0_f64, f64::max));
    }

    // Pass B: apply all changes now that every scale has been determined.
    let output_multiplier = f64::from(productivity_multiplier.max(0.0));
    let mut building_results: Vec<BuildingProductionResult> = Vec::new();
    for mut p in pending {
        let site_mult = site_multiplier_for(site_multipliers, p.building_type);
        // Each line applies at its OWN scale (issue #272) — that independence
        // is the point of lines.
        for line in &mut p.lines {
            if line.scale <= 1e-9 {
                continue;
            }
            for ingredient in &line.recipe.inputs {
                stores.withdraw(&ingredient.id, ingredient.quantity * line.scale);
            }
            for ingredient in &line.recipe.outputs {
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
                let deposited = ingredient.quantity
                    * line.scale
                    * output_multiplier
                    * category_mult
                    * tech_mult
                    * site_mult;
                stores.deposit(&ingredient.id, deposited);
                line.outputs_deposited
                    .push((ingredient.id.clone(), deposited));
            }
        }
        // Upkeep is a building-level cost, charged once regardless of how many
        // lines ran — charging it per line would multiply it by the line count.
        if p.maintenance_scale > 1e-9 {
            for ingredient in p.maintenance {
                stores.withdraw(
                    &ingredient.id,
                    ingredient.quantity * p.maintenance_multiplier * p.maintenance_scale,
                );
            }
        }

        // `recipe_id` / `concurrent_recipe_ids` / `scale` keep their pre-#272
        // meanings so existing consumers are unaffected: the default line's
        // selection, the always-on recipes, and a single headline scale. With
        // independent lines a single scale is necessarily a summary — the
        // *worst* line, so "is anything wrong here?" still answers correctly.
        // `line_results` carries the real per-line detail.
        let recipe_id = p
            .lines
            .iter()
            .find(|l| !l.always_on && l.line.is_none())
            .map(|l| l.recipe.id.clone())
            .unwrap_or_default();
        let concurrent_recipe_ids = p
            .lines
            .iter()
            .filter(|l| l.always_on)
            .map(|l| l.recipe.id.clone())
            .collect();
        let scale = p
            .lines
            .iter()
            .map(|l| l.scale)
            .fold(f64::INFINITY, f64::min);
        let scale = if scale.is_finite() { scale } else { 0.0 };
        let shortfalls: Vec<ProductionShortfall> =
            p.lines.iter().flat_map(|l| l.shortfalls.clone()).collect();
        let line_results = p
            .lines
            .iter()
            .map(|l| LineProductionResult {
                line: l.line.clone(),
                always_on: l.always_on,
                recipe_id: l.recipe.id.clone(),
                scale: l.scale,
                shortfalls: l.shortfalls.clone(),
                outputs_deposited: l.outputs_deposited.clone(),
            })
            .collect();
        building_results.push(BuildingProductionResult {
            building_id: p.building_id,
            building_type: p.building_type.to_owned(),
            recipe_id,
            concurrent_recipe_ids,
            scale,
            shortfalls,
            line_results,
        });
    }

    ProductionStepOutcome {
        building_results,
        power_grid,
        labor_available,
        labor_demanded,
        labour: labour_plan,
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Convert the `f32` labour figure the pipeline carries into a whole-worker count.
///
/// Truncates rather than rounds, so a fractional worker is never handed out as a
/// whole one, and saturates at [`u32::MAX`] so an absurd population can't wrap
/// around to zero workers.
fn workforce(labor_available: f32) -> u32 {
    let whole = labor_available.max(0.0).trunc();
    if whole.is_nan() {
        return 0;
    }
    // `f32` can represent values far beyond u32::MAX; clamp before converting.
    if whole >= 4_294_967_296.0 {
        u32::MAX
    } else {
        // Truncated, non-negative, and below u32::MAX — the conversion is exact.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            whole as u32
        }
    }
}

/// Compute the colony power grid, scaling consumer power draws by
/// `power_scalar` (issue #161). Generators are unaffected. Pass `1.0` for
/// the neutral (no-difficulty) case.
///
/// `pub(crate)` so [`crate::lib`]'s per-sol turn loop can compute each
/// colony's raw capacity/demand *before* running production, to resolve
/// cross-colony powerline transfers (issue #383) ahead of the pass that
/// actually applies them.
/// The site multiplier for one building type, or `1.0` when the caller
/// supplied none — no site data, or a building that declares no scaling.
fn site_multiplier_for(
    site_multipliers: Option<&std::collections::HashMap<String, f64>>,
    building_type: &str,
) -> f64 {
    site_multipliers
        .and_then(|m| m.get(building_type))
        .copied()
        .unwrap_or(1.0)
}

pub(crate) fn compute_power_grid_scaled(
    buildings: &[ProductionInput],
    registry: &ContentRegistry,
    power_scalar: f32,
    active_recipes: &std::collections::HashMap<String, String>,
    site_multipliers: Option<&std::collections::HashMap<String, f64>>,
) -> PowerGrid {
    let mut capacity = 0.0f64;
    let mut demand = 0.0f64;
    let mul = f64::from(power_scalar.max(0.0));

    for input in buildings {
        let building_type = &input.building_type;
        let Some(bdef) = registry.building(building_type) else {
            continue;
        };
        // Negative power_delta = generator.
        if bdef.power_delta < 0.0 {
            // Site scaling (issue #411) applies to grid capacity as well as
            // to recipe output. Scaling only the latter would leave a
            // generator advertising headroom its own output can no longer
            // fill — a half-lit solar array still claiming full capacity.
            capacity += -bdef.power_delta * site_multiplier_for(site_multipliers, building_type);
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

/// Resolve cross-colony power transfer over `Powerline` infrastructure edges
/// (issue #383), given each colony's raw (pre-transfer) [`PowerGrid`].
///
/// For every powerline edge, whichever endpoint has surplus capacity
/// (`capacity > demand`) sends toward whichever has a deficit, capped by the
/// edge's `throughput` and net of its `loss_pct` — the importer receives
/// `sent * (1.0 - loss_pct)`, and the exporter's surplus is drawn down by the
/// full `sent` amount (the loss is a transit cost the exporter's grid still
/// "pays," not the importer's). Edges are resolved in order, so a colony
/// with several powerlines can export to (or import from) more than one
/// neighbor in the same sol, each draining/filling its running balance.
///
/// Returns a per-colony net import: positive means "add this much capacity,"
/// negative means "this colony exported this much of its own surplus" — feed
/// straight into [`process_production_scaled`]'s `power_import` parameter.
/// A colony absent from the edge list, or with no powerline neighbor able to
/// trade, is simply absent from the returned map.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn resolve_power_transfers(
    grids: &std::collections::HashMap<crate::ColonyId, PowerGrid>,
    edges: &[crate::map::InfraEdge],
) -> std::collections::HashMap<crate::ColonyId, f64> {
    let mut balance: std::collections::HashMap<crate::ColonyId, f64> = grids
        .iter()
        .map(|(id, g)| (*id, g.capacity - g.demand))
        .collect();
    let mut net_import: std::collections::HashMap<crate::ColonyId, f64> =
        std::collections::HashMap::new();

    for edge in edges {
        if edge.infra_type != crate::map::InfraType::Powerline {
            continue;
        }
        let (Some(&bal_from), Some(&bal_to)) = (balance.get(&edge.from), balance.get(&edge.to))
        else {
            continue;
        };

        let (exporter, importer, available, needed) = if bal_from > 0.0 && bal_to < 0.0 {
            (edge.from, edge.to, bal_from, -bal_to)
        } else if bal_to > 0.0 && bal_from < 0.0 {
            (edge.to, edge.from, bal_to, -bal_from)
        } else {
            continue;
        };

        let loss = f64::from(edge.loss_pct.clamp(0.0, 0.99));
        let throughput = f64::from(edge.throughput.max(0.0));
        // How much the exporter must send for the importer to receive
        // `needed`, net of loss — capped by both sides' balance and the
        // edge's own throughput.
        let sent = available.min(needed / (1.0 - loss)).min(throughput);
        if sent <= 0.0 {
            continue;
        }
        let delivered = sent * (1.0 - loss);

        *balance.entry(exporter).or_insert(0.0) -= sent;
        *balance.entry(importer).or_insert(0.0) += delivered;
        *net_import.entry(exporter).or_insert(0.0) -= sent;
        *net_import.entry(importer).or_insert(0.0) += delivered;
    }

    net_import
}

/// Sum each resource's banking capacity across every completed building the
/// colony has (issue #348).
///
/// Counts every placed instance regardless of staffing or `paused` state —
/// storage is passive capacity, not something that needs a crew to hold a
/// charge. Buildings with an empty [`crate::content::types::BuildingDef::storage`]
/// (the default) contribute nothing, so a colony with no storage buildings at
/// all gets an empty map back, which is what makes every resource fall back
/// to the pre-#348 "evaporates every sol" behaviour.
#[must_use]
pub fn storage_capacities(
    buildings: &[PlacedBuilding],
    registry: &ContentRegistry,
) -> std::collections::HashMap<String, f64> {
    let mut capacities = std::collections::HashMap::new();
    for placed in buildings {
        let Some(bdef) = registry.building(&placed.building_type) else {
            continue;
        };
        for entry in &bdef.storage {
            *capacities.entry(entry.id.clone()).or_insert(0.0) += entry.quantity;
        }
    }
    capacities
}

/// Scale a colony's habitability modifier down for hex contamination (issue
/// #387), at the point of use rather than baked into
/// [`crate::colony::Colony::habitability_modifier`] itself — contamination
/// changes sol to sol and is a per-hex, per-colony consequence, whereas
/// `habitability_modifier` is the body's own (slower-changing, mitigation-
/// driven) environmental score. Keeping the two factors separate and
/// multiplying them together at the call site avoids double-counting either
/// one into the other.
///
/// `contamination` of `0.0` (pristine) leaves the modifier untouched;
/// `1.0` (maximally contaminated) halves it. Never drives the modifier to
/// zero — even a fully fouled hex still supports some reduced output,
/// consistent with contamination being a penalty, not a colony-killer.
#[must_use]
pub fn contamination_habitability_factor(contamination: f32) -> f32 {
    (1.0 - contamination.clamp(0.0, 1.0) * 0.5).max(0.5)
}

/// Yield fraction an extraction recipe manages with **no matching deposit**
/// present (issue #317).
///
/// Answers #317's first open question — "do you want a local fallback so a colony
/// is never hard-blocked?" — with yes. Deposits are sparse and clustered, so
/// without this a colony founded away from a vein simply cannot enter a chain,
/// and the only remedy is an outpost or a trade route it may not be able to
/// build yet.
///
/// Deliberately well below the `0.5` floor a real deposit guarantees: bare ground
/// keeps a chain *alive*, it does not make prospecting pointless. Balance dial;
/// expect the harness and playtesting to retune it.
pub const TRACE_DEPOSIT_RATIO: f64 = 0.15;

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
/// deposit-gated outputs sets the ratio:
///
/// - **No matching deposit at all** yields [`TRACE_DEPOSIT_RATIO`] rather than
///   zero (issue #317). Scraping a trace yield out of unremarkable ground is the
///   early-game fallback that stops a colony being hard-blocked out of a
///   production chain by where it happened to land.
/// - **Any deposit** guarantees a `0.5` floor, scaling linearly to `1.0` at
///   richness `1.0` — so a guaranteed-placed but low-richness deposit (#232's
///   coverage guarantee) still produces meaningfully more than bare ground, and
///   richness matters rather than being a presence/absence toggle.
///
/// The gap between the trace floor and the deposit floor is what makes finding a
/// deposit worth doing: a real deposit is at least three times the yield.
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
        // Bare ground: a trace yield, not nothing (#317).
        return (TRACE_DEPOSIT_RATIO, Some(worst_commodity.to_string()));
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
/// What a building can afford this sol, and the demands that answer was based on.
struct InputAffordability {
    /// Fraction of demand the pool can cover, in `[0.0, 1.0]`.
    ratio: f64,
    /// The commodity that bound the ratio, if any.
    tight: Option<String>,
    /// Whether the binding commodity was a maintenance draw rather than a recipe
    /// input — they report as different shortfall reasons.
    tight_is_maintenance: bool,
    /// How much more of the binding commodity full output would have needed.
    ///
    /// `0.0` when nothing bound. Carried alongside `ratio` because a percentage
    /// on its own does not tell the player whether they are two units short or
    /// two hundred (issues #303, #308).
    tight_deficit: f64,
    /// Merged recipe inputs at **full rate**, `(commodity_id, quantity)`.
    ///
    /// Returned so the caller can reserve exactly what it was judged against
    /// (issue #308) — reserving a re-derived list risks the two drifting apart.
    ///
    /// Maintenance is deliberately *not* here: upkeep is charged once per
    /// building while inputs are charged per line, and it is a pure function of
    /// the building's authored list, so both this function and the reservation
    /// site derive it from the shared [`maintenance_draws`].
    recipe_demands: Vec<(String, f64)>,
}

/// A building's per-sol maintenance draws at full rate, scaled by `multiplier`.
///
/// Shared by the affordability check and the reservation pass so the two cannot
/// disagree about what upkeep costs (issue #308).
fn maintenance_draws(maintenance: &[Ingredient], multiplier: f64) -> Vec<(String, f64)> {
    let mut out: Vec<(String, f64)> = Vec::new();
    if multiplier <= 0.0 {
        return out;
    }
    for ing in maintenance {
        if ing.quantity <= 0.0 {
            continue;
        }
        let scaled = ing.quantity * multiplier;
        if let Some(existing) = out.iter_mut().find(|d| d.0 == ing.id) {
            existing.1 += scaled;
        } else {
            out.push((ing.id.clone(), scaled));
        }
    }
    out
}

/// How much of `commodity_id` is left after everything already reserved.
fn unreserved(
    stores: &ColonyStores<'_>,
    reserved: &std::collections::HashMap<String, f64>,
    commodity_id: &str,
) -> f64 {
    let held = stores.amount(commodity_id);
    let claimed = reserved.get(commodity_id).copied().unwrap_or(0.0);
    (held - claimed).max(0.0)
}

/// Add `amount` to the running reservation for `commodity_id`.
fn reserve(reserved: &mut std::collections::HashMap<String, f64>, commodity_id: &str, amount: f64) {
    if amount <= 0.0 {
        return;
    }
    *reserved.entry(commodity_id.to_owned()).or_insert(0.0) += amount;
}

/// Judge what a building can run at, against the pool **minus what
/// higher-priority buildings have already claimed** (issue #308).
///
/// `reserved` is what makes competing consumers honest. Before #308 every
/// building was judged against the whole pool independently, so two buildings
/// each needing the colony's last 10 water both concluded they could run at full
/// rate — and both did, the second producing a full batch having consumed
/// nothing, with no shortfall reported. Output was fabricated from an empty pool.
fn compute_effective_input_ratio(
    stores: &ColonyStores<'_>,
    reserved: &std::collections::HashMap<String, f64>,
    recipe: Option<&RecipeDef>,
    concurrent: &[&RecipeDef],
    maintenance: &[Ingredient],
    maintenance_multiplier: f64,
) -> InputAffordability {
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
    for (id, scaled) in maintenance_draws(maintenance, maintenance_multiplier) {
        if let Some(existing) = demands.iter_mut().find(|d| d.0 == id) {
            existing.1 += scaled;
        } else {
            demands.push((id, scaled, false));
        }
    }

    let mut ratio = 1.0f64;
    let mut tight: Option<String> = None;
    let mut tight_is_maintenance = false;
    let mut tight_deficit = 0.0f64;

    for (id, qty, has_recipe_demand) in &demands {
        let available = unreserved(stores, reserved, id);
        let r = (available / *qty).min(1.0);
        if r < ratio {
            ratio = r;
            tight = Some(id.clone());
            tight_is_maintenance = !*has_recipe_demand;
            // How much more of the binding commodity full output would have
            // needed. `effective_scale` alone tells the player they are at 30 %
            // but not whether they are 2 units short or 200 (issue #303/#308).
            tight_deficit = (*qty - available).max(0.0);
        }
    }

    // Split the merged list back out: inputs are charged per line, upkeep once
    // per building, so they have to be reserved separately.
    let mut recipe_demands: Vec<(String, f64)> = Vec::new();
    for r in recipe.into_iter().chain(concurrent.iter().copied()) {
        for ing in &r.inputs {
            if ing.quantity <= 0.0 {
                continue;
            }
            if let Some(existing) = recipe_demands.iter_mut().find(|d| d.0 == ing.id) {
                existing.1 += ing.quantity;
            } else {
                recipe_demands.push((ing.id.clone(), ing.quantity));
            }
        }
    }
    InputAffordability {
        ratio: ratio.max(0.0),
        tight,
        tight_is_maintenance,
        tight_deficit,
        recipe_demands,
    }
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
/// That is the selected recipe of *every* production line — the same set
/// [`run_production`] runs, one per line — so the summary covers the building's
/// whole function rather than one arbitrary recipe. A multi-line building like
/// `fabrication_complex` reports its foundry *and* its machine shop; picking
/// only the lexicographically-first recipe would silently hide the rest.
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
    let mut recipe_ids = Vec::new();
    let mut inputs: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
    let mut outputs: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();

    for line in lines_for_building(building_type, active_recipes, registry) {
        let recipe = line.selected;
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

/// Separator between a building id and a line name in a
/// [`crate::colony::Colony::active_recipes`] key (issue #272).
///
/// ASCII unit separator: it cannot occur in a content-pack id, so a composite
/// key can never collide with a bare `building_type`. That matters because the
/// **default** line keys on the bare `building_type` — exactly how selections
/// were keyed before lines existed — which is what lets pre-#272 saves and
/// `Command::SetActiveRecipe` keep working with no migration at all.
const LINE_KEY_SEP: char = '\u{1f}';

/// Storage key for a line's recipe selection.
///
/// `None` (the default line) → the bare `building_type`, for back-compat.
/// `Some(line)` → a composite key that cannot collide with one.
#[must_use]
pub fn line_selection_key(building_type: &str, line: Option<&str>) -> String {
    match line {
        None => building_type.to_owned(),
        Some(l) => format!("{building_type}{LINE_KEY_SEP}{l}"),
    }
}

/// One independently-throttling production line within a building (issue #272).
///
/// A building's recipes are partitioned into lines. Recipes sharing a line are
/// **alternatives** — exactly one runs, chosen by the player or by the
/// deterministic default. Different lines run **simultaneously**, each computing
/// its own scale from its own inputs, which is what makes them separate
/// production chains rather than one chain plus a set of always-on extras.
#[derive(Debug, Clone)]
pub struct RecipeLine<'r> {
    /// Authored line name; `None` is the building's default line.
    pub line: Option<String>,
    /// `true` for a line derived from a [`RecipeDef::concurrent`] recipe — it has
    /// exactly one member, so there is nothing to choose and it always runs.
    pub always_on: bool,
    /// The recipe currently running on this line.
    pub selected: &'r RecipeDef,
    /// Every recipe on this line, in id order — what a picker would offer.
    /// Length 1 means no real choice.
    pub alternatives: Vec<&'r RecipeDef>,
}

impl RecipeLine<'_> {
    /// Key this line's selection is stored under in `active_recipes`.
    #[must_use]
    pub fn selection_key(&self, building_type: &str) -> String {
        line_selection_key(building_type, self.line.as_deref())
    }
}

/// Partition a building's recipes into independently-running lines (issue #272).
///
/// - A [`RecipeDef::concurrent`] recipe becomes a line of its own, always on.
///   This reproduces the pre-#272 rule ("concurrent recipes always run") exactly,
///   now expressed in the same vocabulary as everything else.
/// - Every other recipe joins the line named by [`RecipeDef::line`], or the
///   default line when that is `None`. One recipe per line runs: the selection in
///   `active_recipes` if it is valid for that line, else the lexicographically
///   smallest recipe id on the line (the same deterministic default as before —
///   `ContentRegistry` iterates a `HashMap`, so an arbitrary "first" would vary
///   between runs).
///
/// Lines come back in a deterministic order: the default line first, then named
/// lines and always-on lines by name.
#[must_use]
pub fn lines_for_building<'r, S: std::hash::BuildHasher>(
    building_type: &str,
    active_recipes: &std::collections::HashMap<String, String, S>,
    registry: &'r ContentRegistry,
) -> Vec<RecipeLine<'r>> {
    // Group non-concurrent recipes by line, and collect concurrent ones apart.
    let mut grouped: std::collections::BTreeMap<Option<String>, Vec<&RecipeDef>> =
        std::collections::BTreeMap::new();
    let mut always_on: Vec<&RecipeDef> = Vec::new();

    for recipe in registry.recipes().filter(|r| r.building == building_type) {
        if recipe.concurrent && recipe.line.is_none() {
            always_on.push(recipe);
        } else {
            grouped.entry(recipe.line.clone()).or_default().push(recipe);
        }
    }

    let mut lines: Vec<RecipeLine<'r>> = Vec::new();

    for (line, mut alternatives) in grouped {
        alternatives.sort_by(|a, b| a.id.cmp(&b.id));
        let key = line_selection_key(building_type, line.as_deref());
        // A selection only counts if it names a recipe actually on this line —
        // otherwise a stale save or a cross-line id would silently run nothing.
        let selected = active_recipes
            .get(&key)
            .and_then(|id| alternatives.iter().copied().find(|r| &r.id == id))
            .or_else(|| alternatives.first().copied());
        if let Some(selected) = selected {
            lines.push(RecipeLine {
                line,
                always_on: false,
                selected,
                alternatives,
            });
        }
    }

    always_on.sort_by(|a, b| a.id.cmp(&b.id));
    for recipe in always_on {
        lines.push(RecipeLine {
            line: Some(recipe.id.clone()),
            always_on: true,
            selected: recipe,
            alternatives: vec![recipe],
        });
    }

    lines
}

/// Returns true if there is at least one recipe (pick-one or concurrent)
/// for the given building type.
pub(crate) fn has_recipe(building_type: &str, registry: &ContentRegistry) -> bool {
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

    // ── Contamination habitability factor (issue #387) ───────────────────────

    #[test]
    fn contamination_habitability_factor_is_neutral_at_zero() {
        assert!((contamination_habitability_factor(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn contamination_habitability_factor_halves_at_full_severity() {
        assert!((contamination_habitability_factor(1.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn contamination_habitability_factor_is_linear_between() {
        assert!((contamination_habitability_factor(0.5) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn contamination_habitability_factor_clamps_out_of_range_input() {
        assert!((contamination_habitability_factor(-1.0) - 1.0).abs() < 1e-6);
        assert!((contamination_habitability_factor(2.0) - 0.5).abs() < 1e-6);
    }

    // ── Cross-colony power transfer (issue #383) ──────────────────────────────

    fn powerline_edge(
        from: Uuid,
        to: Uuid,
        throughput: f32,
        loss_pct: f32,
    ) -> crate::map::InfraEdge {
        crate::map::InfraEdge {
            from,
            to,
            infra_type: crate::map::InfraType::Powerline,
            cost: 0.0,
            throughput,
            loss_pct,
        }
    }

    #[test]
    fn resolve_power_transfers_moves_surplus_to_deficit_net_of_loss() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let grids = std::collections::HashMap::from([
            (
                a,
                PowerGrid {
                    capacity: 100.0,
                    demand: 20.0,
                },
            ), // 80 surplus
            (
                b,
                PowerGrid {
                    capacity: 10.0,
                    demand: 50.0,
                },
            ), // 40 deficit
        ]);
        let edges = vec![powerline_edge(a, b, 1000.0, 0.5)];

        let imports = resolve_power_transfers(&grids, &edges);

        // b needs 40, net of 50% loss the exporter must send 80 to deliver 40 —
        // a's surplus (80) exactly covers it.
        assert!((imports[&b] - 40.0).abs() < 1e-6);
        assert!((imports[&a] - (-80.0)).abs() < 1e-6);
    }

    #[test]
    fn resolve_power_transfers_caps_at_edge_throughput() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let grids = std::collections::HashMap::from([
            (
                a,
                PowerGrid {
                    capacity: 1000.0,
                    demand: 0.0,
                },
            ), // huge surplus
            (
                b,
                PowerGrid {
                    capacity: 0.0,
                    demand: 1000.0,
                },
            ), // huge deficit
        ]);
        let edges = vec![powerline_edge(a, b, 50.0, 0.0)];

        let imports = resolve_power_transfers(&grids, &edges);

        // Capped by the edge's throughput, not by either colony's balance.
        assert!((imports[&b] - 50.0).abs() < 1e-6);
        assert!((imports[&a] - (-50.0)).abs() < 1e-6);
    }

    #[test]
    fn resolve_power_transfers_ignores_non_powerline_edges() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let grids = std::collections::HashMap::from([
            (
                a,
                PowerGrid {
                    capacity: 100.0,
                    demand: 0.0,
                },
            ),
            (
                b,
                PowerGrid {
                    capacity: 0.0,
                    demand: 100.0,
                },
            ),
        ]);
        let mut edge = powerline_edge(a, b, 1000.0, 0.0);
        edge.infra_type = crate::map::InfraType::Road;
        let edges = vec![edge];

        let imports = resolve_power_transfers(&grids, &edges);
        assert!(imports.is_empty(), "a road edge must not carry power");
    }

    #[test]
    fn resolve_power_transfers_no_transfer_when_both_have_surplus() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let grids = std::collections::HashMap::from([
            (
                a,
                PowerGrid {
                    capacity: 100.0,
                    demand: 10.0,
                },
            ),
            (
                b,
                PowerGrid {
                    capacity: 100.0,
                    demand: 10.0,
                },
            ),
        ]);
        let edges = vec![powerline_edge(a, b, 1000.0, 0.0)];

        let imports = resolve_power_transfers(&grids, &edges);
        assert!(
            imports.is_empty(),
            "no deficit colony means nothing to transfer"
        );
    }

    use crate::colony::{ColonyPool, ColonyResourcePool};
    use crate::content::types::{
        BuildingCategory, BuildingDef, Ingredient, RecipeDef, DEFAULT_BUILDING_PRIORITY,
    };
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
            &ProductionInput::from_types(buildings),
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
            &ProductionInput::from_types(buildings),
            labor_available,
            registry,
            power_scalar,
            0.0, // power_import (issue #383)
            maintenance_scalar,
            maintenance_enabled,
            productivity_multiplier,
            active_recipes,
            category_modifiers,
            deposit_richness,
            modifier_accumulator,
            difficulty_scalar,
            // Tests that care about site scaling call the real function
            // directly; this shared helper keeps its existing 23 call sites
            // unchanged by passing no site data (issue #411).
            None,
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
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
            line: None,
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
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
            line: None,
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

    /// A labour shortage is **concentrated, not spread** (issue #307).
    ///
    /// This test previously asserted the opposite: a colony-wide
    /// `available / demanded` ratio ran *every* building at 40%. Per-building
    /// allocation replaces that — the workforce is handed out in priority order
    /// and a building either gets its staff or doesn't. Two half-fed buildings
    /// producing nothing useful was the exact failure mode #307 set out to fix.
    #[test]
    fn labor_short_starves_one_building_rather_than_throttling_both() {
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 100.0);

        // mine wants 2 workers, smelter wants 3 → 5 demanded, only 2 available.
        // Both sit at the default priority, so the deterministic tiebreak decides:
        // "mine" sorts before "smelter", and 2 workers is exactly its demand.
        let placed = buildings(&["solar_array", "mine", "smelter"]);
        let outcome = process_production(&mut pool, &placed, 2.0_f32, &reg);

        let result = |building_type: &str| {
            outcome
                .building_results
                .iter()
                .find(|r| r.building_type == building_type)
                .expect("building present in results")
        };

        let mine = result("mine");
        assert!(
            (mine.scale - 1.0).abs() < 1e-6,
            "the fully-staffed mine runs at full rate, got {}",
            mine.scale
        );
        assert!(
            !mine
                .shortfalls
                .iter()
                .any(|s| s.reason == ShortfallReason::LaborShort),
            "a fully-staffed building is not labour-short"
        );

        let smelter = result("smelter");
        assert!(
            smelter.scale.abs() < 1e-6,
            "the unstaffed smelter runs at zero, got {}",
            smelter.scale
        );
        assert!(
            smelter
                .shortfalls
                .iter()
                .any(|s| s.reason == ShortfallReason::LaborShort),
            "expected LaborShort for the unstaffed smelter"
        );

        // The old colony-wide ratio would have put both at 0.4.
        assert!(
            (mine.scale - smelter.scale).abs() > 0.5,
            "the shortage must land unevenly"
        );

        // And the plan agrees with the scales it produced.
        assert_eq!(outcome.labor_demanded, 5.0);
        assert_eq!(outcome.labour.idle, 0);
        assert_eq!(outcome.labour.unfilled, 3, "the smelter's three empty jobs");
    }

    #[test]
    fn labor_short_reports_the_shortfall_on_every_starved_building() {
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 100.0);

        // Zero workers: nothing can be staffed, so both report labour-short.
        let placed = buildings(&["solar_array", "mine", "smelter"]);
        let outcome = process_production(&mut pool, &placed, 0.0_f32, &reg);

        for res in &outcome.building_results {
            if res.building_type == "mine" || res.building_type == "smelter" {
                assert!(
                    res.scale.abs() < 1e-6,
                    "building {} should be idle, got {}",
                    res.building_type,
                    res.scale
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

    // ── Per-building labour steering (issue #307) ────────────────────────────

    /// Build a production input at an explicit priority. Ids are index-derived so
    /// the allocator's tiebreak stays reproducible across runs.
    fn input_at(index: u128, building_type: &str, priority: u8) -> ProductionInput {
        ProductionInput {
            id: Uuid::from_u128(index),
            building_type: building_type.to_owned(),
            slot_cost: 1,
            priority,
            labour_lock: None,
        }
    }

    fn run_inputs(
        pool: &mut ColonyPool,
        inputs: &[ProductionInput],
        labor: f32,
        reg: &ContentRegistry,
    ) -> ProductionStepOutcome {
        let mut resources = ColonyResourcePool::new();
        super::process_production_scaled(
            &mut ColonyStores::new(pool, &mut resources, reg),
            inputs,
            labor,
            reg,
            1.0,
            0.0, // power_import (issue #383)
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
            None,
        )
    }

    /// Priority, not name order or build order, decides who gets staffed.
    #[test]
    fn a_better_priority_is_staffed_first_through_the_full_pipeline() {
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 100.0);

        // The smelter has the better priority despite sorting later by name — so
        // if priority were ignored, the alphabetical tiebreak would staff the mine
        // and this test would fail.
        let inputs = vec![
            input_at(0, "solar_array", DEFAULT_BUILDING_PRIORITY),
            input_at(1, "mine", 9),
            input_at(2, "smelter", 1),
        ];
        // 3 workers: exactly the smelter's demand, leaving the mine nothing.
        let outcome = run_inputs(&mut pool, &inputs, 3.0, &reg);

        let scale = |bt: &str| {
            outcome
                .building_results
                .iter()
                .find(|r| r.building_type == bt)
                .map(|r| r.scale)
                .expect("building present")
        };
        assert!(
            (scale("smelter") - 1.0).abs() < 1e-6,
            "priority 1 runs full"
        );
        assert!(
            scale("mine").abs() < 1e-6,
            "priority 9 gets what's left: none"
        );
    }

    /// A building that cannot run for a *non-labour* reason releases its workers.
    ///
    /// This is why labour is allocated after input/power/deposit resolution rather
    /// than alongside it: holding staff at a mine with nothing to dig would strand
    /// them where they can accomplish nothing.
    #[test]
    fn a_building_that_cannot_run_releases_its_labour_to_a_worse_priority() {
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        // No ore at all, so the smelter is input-starved to a standstill.
        assert_eq!(pool.amount("iron_ore"), 0.0);

        let inputs = vec![
            input_at(0, "solar_array", DEFAULT_BUILDING_PRIORITY),
            // The smelter has the *better* priority and would otherwise take all
            // three workers, leaving the mine idle and the colony with nothing.
            input_at(1, "smelter", 1),
            input_at(2, "mine", 9),
        ];
        let outcome = run_inputs(&mut pool, &inputs, 3.0, &reg);

        let smelter = outcome.labour.for_building(inputs[1].id).expect("smelter");
        assert_eq!(
            smelter.demand, 0,
            "a building with no inputs offers no jobs this sol"
        );
        assert_eq!(smelter.assigned, 0);

        let mine = outcome.labour.for_building(inputs[2].id).expect("mine");
        assert_eq!(
            mine.assigned, 2,
            "the freed workers staffed the mine despite its worse priority"
        );
        assert_eq!(outcome.labour.idle, 1, "3 workers, only 2 slots to fill");

        // And the mine really did produce, rather than merely being allocated to.
        assert!(pool.amount("iron_ore") > 0.0, "the mine ran");
    }

    /// A lock survives the full pipeline, not just the allocator.
    #[test]
    fn a_labour_lock_holds_staff_against_a_better_priority() {
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 100.0);

        let mut locked_mine = input_at(1, "mine", 9);
        locked_mine.labour_lock = Some(2);
        let inputs = vec![
            input_at(0, "solar_array", DEFAULT_BUILDING_PRIORITY),
            locked_mine,
            input_at(2, "smelter", 1),
        ];
        // 3 workers: the lock claims 2 first, so the priority-1 smelter — which
        // wants 3 — is left with a single worker.
        let outcome = run_inputs(&mut pool, &inputs, 3.0, &reg);

        let mine = outcome.labour.for_building(inputs[1].id).expect("mine");
        assert_eq!(mine.assigned, 2, "the lock is honoured first");
        assert!(mine.locked);

        let smelter = outcome.labour.for_building(inputs[2].id).expect("smelter");
        assert_eq!(smelter.assigned, 1, "and gets only the remainder");
        let smelter_scale = outcome
            .building_results
            .iter()
            .find(|r| r.building_type == "smelter")
            .map(|r| r.scale)
            .expect("smelter result");
        assert!(
            (smelter_scale - 1.0 / 3.0).abs() < 1e-6,
            "one of three workers means a third of the output, got {smelter_scale}"
        );
    }

    /// Two instances of one type can now run at different scales — the thing the
    /// old colony-wide ratio made impossible.
    #[test]
    fn two_instances_of_a_type_are_staffed_independently() {
        let reg = make_registry_with_power();
        let mut pool = ColonyPool::new();

        let inputs = vec![
            input_at(0, "solar_array", DEFAULT_BUILDING_PRIORITY),
            input_at(1, "mine", 1),
            input_at(2, "mine", 9),
        ];
        // 2 workers: exactly one mine's worth.
        let outcome = run_inputs(&mut pool, &inputs, 2.0, &reg);

        let first = outcome.labour.for_building(inputs[1].id).expect("mine 1");
        let second = outcome.labour.for_building(inputs[2].id).expect("mine 2");
        assert_eq!(
            first.assigned, 2,
            "the better-priority mine is fully staffed"
        );
        assert_eq!(second.assigned, 0, "its sibling gets nothing");

        // And the results keep them apart too, keyed by instance — one mine ran,
        // the other didn't. A type-keyed map could not express this.
        let run_for = |id| {
            outcome
                .building_results
                .iter()
                .find(|r| r.building_id == id)
                .expect("a result per instance")
        };
        assert!(run_for(inputs[1].id).scale > 0.0, "the staffed mine ran");
        assert!(
            run_for(inputs[2].id).scale.abs() < 1e-6,
            "the unstaffed mine did not"
        );
    }

    // ── Competing consumers of one scarce input (issue #308) ─────────────────

    /// Two buildings that each consume 10 water and produce 1 widget, plus a
    /// generator so power is never the constraint.
    fn make_registry_with_two_water_consumers() -> ContentRegistry {
        let mut reg = ContentRegistry::default();
        let building = |id: &str| BuildingDef {
            id: id.into(),
            name: id.into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 0,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
            default_priority: DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
        };
        for id in ["eater_a", "eater_b"] {
            reg.insert_building(building(id));
            reg.insert_recipe(RecipeDef {
                id: format!("{id}_run"),
                name: format!("{id}_run"),
                building: id.into(),
                cycle_sols: 1,
                inputs: vec![Ingredient {
                    id: "water".into(),
                    quantity: 10.0,
                }],
                outputs: vec![Ingredient {
                    id: "widget".into(),
                    quantity: 1.0,
                }],
                concurrent: false,
                line: None,
                power_draw: 0.0,
            });
        }
        reg
    }

    /// The bug #308 was really about: output fabricated from an empty pool.
    ///
    /// Both buildings used to be judged against the whole pool independently, so
    /// each concluded it could run at full rate on the colony's last 10 water.
    /// Both then ran at 1.0 and **two** widgets came out of enough water for one
    /// — the second consuming nothing, with no shortfall reported.
    #[test]
    fn two_consumers_cannot_both_spend_the_same_scarce_input() {
        let reg = make_registry_with_two_water_consumers();
        let mut pool = ColonyPool::new();
        pool.deposit("water", 10.0); // enough for exactly ONE of the two

        let inputs = vec![
            input_at(0, "eater_a", DEFAULT_BUILDING_PRIORITY),
            input_at(1, "eater_b", DEFAULT_BUILDING_PRIORITY),
        ];
        let outcome = run_inputs(&mut pool, &inputs, 100.0, &reg);

        // The headline invariant: 10 water buys 1 widget, never 2.
        assert!(
            (pool.amount("widget") - 1.0).abs() < 1e-9,
            "10 water must yield exactly 1 widget, got {}",
            pool.amount("widget")
        );
        assert!(
            pool.amount("water").abs() < 1e-9,
            "all the water should be spent, got {} left",
            pool.amount("water")
        );

        // And the scales agree with that: one ran, one didn't.
        let scales: Vec<f64> = outcome.building_results.iter().map(|r| r.scale).collect();
        let total: f64 = scales.iter().sum();
        assert!(
            (total - 1.0).abs() < 1e-9,
            "combined scale must match the one batch the water affords, got {scales:?}"
        );
    }

    /// The starved building says so, rather than reporting a silent zero.
    #[test]
    fn the_consumer_that_loses_the_input_reports_an_input_shortfall() {
        let reg = make_registry_with_two_water_consumers();
        let mut pool = ColonyPool::new();
        pool.deposit("water", 10.0);

        let inputs = vec![input_at(0, "eater_a", 1), input_at(1, "eater_b", 9)];
        let outcome = run_inputs(&mut pool, &inputs, 100.0, &reg);

        let result = |bt: &str| {
            outcome
                .building_results
                .iter()
                .find(|r| r.building_type == bt)
                .expect("both buildings reported")
        };
        let starved = result("eater_b");
        assert!(starved.scale.abs() < 1e-9);
        assert!(
            starved.shortfalls.iter().any(|s| matches!(
                &s.reason,
                ShortfallReason::InputShort { commodity_id } if commodity_id == "water"
            )),
            "expected an InputShort(water) on the starved building, got {:?}",
            starved.shortfalls
        );
        // And the fed one is not falsely flagged.
        assert!(result("eater_a").shortfalls.is_empty());
    }

    /// Priority decides who eats — the same lever that steers labour (#307).
    ///
    /// The better priority is given to the alphabetically-*later* building, so
    /// passing requires priority to actually be read rather than the deterministic
    /// name tiebreak producing the expected answer on its own.
    #[test]
    fn priority_decides_which_consumer_gets_the_scarce_input() {
        let reg = make_registry_with_two_water_consumers();
        let mut pool = ColonyPool::new();
        pool.deposit("water", 10.0);

        let inputs = vec![input_at(0, "eater_a", 9), input_at(1, "eater_b", 1)];
        let outcome = run_inputs(&mut pool, &inputs, 100.0, &reg);

        let scale = |bt: &str| {
            outcome
                .building_results
                .iter()
                .find(|r| r.building_type == bt)
                .map(|r| r.scale)
                .expect("present")
        };
        assert!(
            (scale("eater_b") - 1.0).abs() < 1e-9,
            "the priority-1 building runs, got {}",
            scale("eater_b")
        );
        assert!(
            scale("eater_a").abs() < 1e-9,
            "the priority-9 building goes without, got {}",
            scale("eater_a")
        );
    }

    // ── Filling vs starved (issue #308) ──────────────────────────────────────

    /// A miner producing `ore` and a smelter consuming it.
    fn make_registry_with_a_chain() -> ContentRegistry {
        let mut reg = ContentRegistry::default();
        let building = |id: &str| BuildingDef {
            id: id.into(),
            name: id.into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 0,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
            default_priority: DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
        };
        reg.insert_building(building("miner"));
        reg.insert_recipe(RecipeDef {
            id: "mine".into(),
            name: "mine".into(),
            building: "miner".into(),
            cycle_sols: 1,
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "ore".into(),
                quantity: 10.0,
            }],
            concurrent: false,
            line: None,
            power_draw: 0.0,
        });
        reg.insert_building(building("smelter"));
        reg.insert_recipe(RecipeDef {
            id: "smelt".into(),
            name: "smelt".into(),
            building: "smelter".into(),
            cycle_sols: 1,
            inputs: vec![Ingredient {
                id: "ore".into(),
                quantity: 10.0,
            }],
            outputs: vec![Ingredient {
                id: "metal".into(),
                quantity: 1.0,
            }],
            concurrent: false,
            line: None,
            power_draw: 0.0,
        });
        reg
    }

    fn reason_for(outcome: &ProductionStepOutcome, building_type: &str) -> ShortfallReason {
        outcome
            .building_results
            .iter()
            .find(|r| r.building_type == building_type)
            .expect("building reported")
            .shortfalls
            .first()
            .expect("a shortfall was recorded")
            .reason
            .clone()
    }

    /// Sol 1 of a fresh chain: the smelter is short, but the ore is produced
    /// here, so this is a pipeline filling — not a missing supply line.
    ///
    /// Production reads a start-of-turn snapshot on purpose, so the first sol of
    /// any chain looks like a shortage. Reporting `InputShort` there invited the
    /// player to build a second mine they did not need.
    #[test]
    fn a_filling_pipeline_reports_awaiting_upstream_not_input_short() {
        let reg = make_registry_with_a_chain();
        let mut pool = ColonyPool::new();
        let inputs = vec![input_at(0, "miner", 5), input_at(1, "smelter", 5)];

        let outcome = run_inputs(&mut pool, &inputs, 100.0, &reg);

        assert!(
            matches!(
                reason_for(&outcome, "smelter"),
                ShortfallReason::AwaitingUpstream { ref commodity_id } if commodity_id == "ore"
            ),
            "expected AwaitingUpstream(ore), got {:?}",
            reason_for(&outcome, "smelter")
        );
        // And it resolves on its own: sol 2 runs at full rate with no shortfall.
        let next = run_inputs(&mut pool, &inputs, 100.0, &reg);
        let smelter = next
            .building_results
            .iter()
            .find(|r| r.building_type == "smelter")
            .unwrap();
        assert!((smelter.scale - 1.0).abs() < 1e-9);
        assert!(smelter.shortfalls.is_empty());
    }

    /// With no local producer of the input, the same shortage is a real supply
    /// problem and still says so.
    #[test]
    fn a_consumer_with_no_local_producer_still_reports_input_short() {
        let reg = make_registry_with_a_chain();
        let mut pool = ColonyPool::new();
        // The smelter stands alone — nothing here makes ore.
        let inputs = vec![input_at(0, "smelter", 5)];

        let outcome = run_inputs(&mut pool, &inputs, 100.0, &reg);

        assert!(
            matches!(
                reason_for(&outcome, "smelter"),
                ShortfallReason::InputShort { ref commodity_id } if commodity_id == "ore"
            ),
            "expected InputShort(ore), got {:?}",
            reason_for(&outcome, "smelter")
        );
    }

    /// Membership is based on what the colony is *configured* to produce, not on
    /// what it managed to produce this sol.
    ///
    /// A mine that is itself starved is still the answer to "where does ore come
    /// from here", so the smelter should point upstream rather than claim there is
    /// no source. Following `AwaitingUpstream` up the chain is how the player
    /// finds the level that reports the real cause.
    #[test]
    fn an_idle_local_producer_still_counts_as_the_source() {
        let mut reg = make_registry_with_a_chain();
        // Re-point the miner at a recipe it cannot run: now it needs fuel that
        // nothing here makes, so it produces no ore at all.
        reg.insert_recipe(RecipeDef {
            id: "mine".into(),
            name: "mine".into(),
            building: "miner".into(),
            cycle_sols: 1,
            inputs: vec![Ingredient {
                id: "fuel".into(),
                quantity: 5.0,
            }],
            outputs: vec![Ingredient {
                id: "ore".into(),
                quantity: 10.0,
            }],
            concurrent: false,
            line: None,
            power_draw: 0.0,
        });

        let mut pool = ColonyPool::new();
        let inputs = vec![input_at(0, "miner", 5), input_at(1, "smelter", 5)];
        let outcome = run_inputs(&mut pool, &inputs, 100.0, &reg);

        // The smelter points upstream …
        assert!(
            matches!(
                reason_for(&outcome, "smelter"),
                ShortfallReason::AwaitingUpstream { ref commodity_id } if commodity_id == "ore"
            ),
            "expected AwaitingUpstream(ore) at the smelter, got {:?}",
            reason_for(&outcome, "smelter")
        );
        // … and the miner is where the chain stops, naming the real cause.
        assert!(
            matches!(
                reason_for(&outcome, "miner"),
                ShortfallReason::InputShort { ref commodity_id } if commodity_id == "fuel"
            ),
            "expected InputShort(fuel) at the miner, got {:?}",
            reason_for(&outcome, "miner")
        );
    }

    /// The shortfall says *how much* is missing, not just how far output fell.
    #[test]
    fn a_shortfall_names_the_missing_quantity() {
        let reg = make_registry_with_two_water_consumers();
        let mut pool = ColonyPool::new();
        pool.deposit("water", 3.0); // recipe wants 10

        let inputs = vec![input_at(0, "eater_a", DEFAULT_BUILDING_PRIORITY)];
        let outcome = run_inputs(&mut pool, &inputs, 100.0, &reg);

        let shortfall = &outcome.building_results[0].shortfalls[0];
        assert!(
            (shortfall.deficit - 7.0).abs() < 1e-9,
            "10 demanded against 3 held is a deficit of 7, got {}",
            shortfall.deficit
        );
        assert!((shortfall.effective_scale - 0.3).abs() < 1e-9);
    }

    /// Shortfalls with no commodity to quantify carry no deficit rather than a
    /// misleading zero-that-means-something.
    #[test]
    fn non_commodity_shortfalls_carry_no_deficit() {
        let reg = make_registry_with_two_water_consumers();
        let mut pool = ColonyPool::new();
        pool.deposit("water", 100.0);

        // Zero workforce: the building is labour-starved, not input-starved.
        let inputs = vec![input_at(0, "eater_a", DEFAULT_BUILDING_PRIORITY)];
        let outcome = run_inputs(&mut pool, &inputs, 0.0, &reg);

        for s in &outcome.building_results[0].shortfalls {
            if matches!(s.reason, ShortfallReason::LaborShort) {
                assert!(
                    s.deficit.abs() < f64::EPSILON,
                    "labour shortfalls have no commodity deficit, got {}",
                    s.deficit
                );
            }
        }
    }

    // ── Player commodity reserves (issue #308) ───────────────────────────────

    fn run_with_reserves(
        pool: &mut ColonyPool,
        inputs: &[ProductionInput],
        reg: &ContentRegistry,
        reserves: &std::collections::HashMap<String, f64>,
    ) -> ProductionStepOutcome {
        let mut resources = ColonyResourcePool::new();
        super::process_production_scaled(
            &mut ColonyStores::new(pool, &mut resources, reg).with_reserves(reserves),
            inputs,
            100.0,
            reg,
            1.0,
            0.0, // power_import (issue #383)
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
            None,
        )
    }

    /// A reserve withholds stock from industry: the consumer sees only what is
    /// above the floor, and the reserved amount is still in the pool afterwards.
    #[test]
    fn a_reserve_keeps_stock_out_of_production() {
        let reg = make_registry_with_two_water_consumers();
        let mut pool = ColonyPool::new();
        pool.deposit("water", 10.0); // exactly one batch's worth

        let mut reserves = std::collections::HashMap::new();
        reserves.insert("water".to_string(), 10.0); // …all of it withheld

        let inputs = vec![input_at(0, "eater_a", DEFAULT_BUILDING_PRIORITY)];
        let outcome = run_with_reserves(&mut pool, &inputs, &reg, &reserves);

        assert!(
            outcome.building_results[0].scale.abs() < 1e-9,
            "a fully reserved input must leave the consumer idle, got {}",
            outcome.building_results[0].scale
        );
        assert!(
            (pool.amount("water") - 10.0).abs() < 1e-9,
            "reserved water must remain in the pool, got {}",
            pool.amount("water")
        );
        assert!(
            pool.amount("widget").abs() < 1e-9,
            "nothing should have been produced from reserved stock"
        );
        assert!(
            outcome.building_results[0]
                .shortfalls
                .iter()
                .any(|s| matches!(
                    &s.reason,
                    ShortfallReason::InputShort { commodity_id } if commodity_id == "water"
                )),
            "the idled building must say which input it lacked, got {:?}",
            outcome.building_results[0].shortfalls
        );
    }

    /// Only the amount above the floor is spendable — a reserve is a floor, not
    /// an all-or-nothing switch.
    #[test]
    fn a_partial_reserve_leaves_the_surplus_spendable() {
        let reg = make_registry_with_two_water_consumers();
        let mut pool = ColonyPool::new();
        pool.deposit("water", 30.0);

        let mut reserves = std::collections::HashMap::new();
        reserves.insert("water".to_string(), 20.0); // 10 spendable = one batch

        let inputs = vec![
            input_at(0, "eater_a", 1),
            input_at(1, "eater_b", 9), // should go without
        ];
        let outcome = run_with_reserves(&mut pool, &inputs, &reg, &reserves);

        assert!(
            (pool.amount("widget") - 1.0).abs() < 1e-9,
            "the 10 unreserved water buys exactly one widget, got {}",
            pool.amount("widget")
        );
        assert!(
            (pool.amount("water") - 20.0).abs() < 1e-9,
            "the floor must be intact, got {}",
            pool.amount("water")
        );
        let scale = |bt: &str| {
            outcome
                .building_results
                .iter()
                .find(|r| r.building_type == bt)
                .map(|r| r.scale)
                .expect("both reported")
        };
        assert!((scale("eater_a") - 1.0).abs() < 1e-9);
        assert!(scale("eater_b").abs() < 1e-9);
    }

    /// Regression guard on the opt-in: a caller that attaches no reserves must
    /// behave exactly as before the feature existed.
    #[test]
    fn no_reserves_attached_changes_nothing() {
        let reg = make_registry_with_two_water_consumers();
        let inputs = vec![input_at(0, "eater_a", DEFAULT_BUILDING_PRIORITY)];

        let mut without = ColonyPool::new();
        without.deposit("water", 10.0);
        let plain = run_inputs(&mut without, &inputs, 100.0, &reg);

        let mut with_empty = ColonyPool::new();
        with_empty.deposit("water", 10.0);
        let empty_reserves = std::collections::HashMap::new();
        let seeded = run_with_reserves(&mut with_empty, &inputs, &reg, &empty_reserves);

        assert!((plain.building_results[0].scale - 1.0).abs() < 1e-9);
        assert!(
            (plain.building_results[0].scale - seeded.building_results[0].scale).abs() < 1e-9,
            "an empty reserve map must be indistinguishable from none"
        );
        assert!((without.amount("widget") - with_empty.amount("widget")).abs() < 1e-9);
    }

    /// A reserve that exceeds what is held is clamped by the availability
    /// arithmetic rather than producing a negative allowance.
    #[test]
    fn a_reserve_larger_than_the_stockpile_is_harmless() {
        let reg = make_registry_with_two_water_consumers();
        let mut pool = ColonyPool::new();
        pool.deposit("water", 5.0);

        let mut reserves = std::collections::HashMap::new();
        reserves.insert("water".to_string(), 1_000.0);

        let inputs = vec![input_at(0, "eater_a", DEFAULT_BUILDING_PRIORITY)];
        let outcome = run_with_reserves(&mut pool, &inputs, &reg, &reserves);

        assert!(outcome.building_results[0].scale.abs() < 1e-9);
        assert!((pool.amount("water") - 5.0).abs() < 1e-9);
    }

    /// Maintenance is deliberately **not** exempt from a reserve.
    ///
    /// A player who reserves the commodity their upkeep runs on can stall their
    /// own buildings. That is a real choice with a `MaintenanceShort` shortfall to
    /// explain it — exempting maintenance would mean two different "available"
    /// figures inside one affordability ratio. Pinned here because the behaviour
    /// is asserted in prose elsewhere and would otherwise be free to drift.
    #[test]
    fn a_reserve_can_stall_maintenance_and_says_so() {
        let reg = make_registry_with_maintenance();
        let mut pool = ColonyPool::new();
        pool.deposit("spare_parts", 10.0); // ample upkeep …

        let mut reserves = std::collections::HashMap::new();
        reserves.insert("spare_parts".to_string(), 10.0); // … all of it withheld

        let inputs = vec![input_at(0, "advanced_smelter", DEFAULT_BUILDING_PRIORITY)];
        let outcome = run_with_reserves(&mut pool, &inputs, &reg, &reserves);

        let result = &outcome.building_results[0];
        assert!(
            result.scale.abs() < 1e-9,
            "upkeep it cannot reach must stop the building, got scale {}",
            result.scale
        );
        assert!(
            result.shortfalls.iter().any(|s| matches!(
                &s.reason,
                ShortfallReason::MaintenanceShort { commodity_id } if commodity_id == "spare_parts"
            )),
            "expected MaintenanceShort(spare_parts), got {:?}",
            result.shortfalls
        );
        assert!(
            (pool.amount("spare_parts") - 10.0).abs() < 1e-9,
            "the reserve must be intact, got {}",
            pool.amount("spare_parts")
        );
    }

    /// A shortage lands unevenly, but a partial remainder is still handed over —
    /// the loser gets whatever the winner left rather than nothing.
    #[test]
    fn the_worse_priority_consumer_gets_the_remainder() {
        let reg = make_registry_with_two_water_consumers();
        let mut pool = ColonyPool::new();
        pool.deposit("water", 15.0); // one full batch plus half of another

        let inputs = vec![input_at(0, "eater_a", 1), input_at(1, "eater_b", 5)];
        let outcome = run_inputs(&mut pool, &inputs, 100.0, &reg);

        let scale = |bt: &str| {
            outcome
                .building_results
                .iter()
                .find(|r| r.building_type == bt)
                .map(|r| r.scale)
                .expect("present")
        };
        assert!((scale("eater_a") - 1.0).abs() < 1e-9);
        assert!(
            (scale("eater_b") - 0.5).abs() < 1e-9,
            "the remainder is half a batch, got {}",
            scale("eater_b")
        );
        assert!(
            (pool.amount("widget") - 1.5).abs() < 1e-9,
            "15 water buys 1.5 widgets, got {}",
            pool.amount("widget")
        );
    }

    /// Reservation must not throttle anything when there is plenty to go round.
    #[test]
    fn ample_input_still_lets_every_consumer_run_at_full_rate() {
        let reg = make_registry_with_two_water_consumers();
        let mut pool = ColonyPool::new();
        pool.deposit("water", 1000.0);

        let inputs = vec![
            input_at(0, "eater_a", DEFAULT_BUILDING_PRIORITY),
            input_at(1, "eater_b", DEFAULT_BUILDING_PRIORITY),
        ];
        let outcome = run_inputs(&mut pool, &inputs, 100.0, &reg);

        assert!(outcome
            .building_results
            .iter()
            .all(|r| (r.scale - 1.0).abs() < 1e-9));
        assert!((pool.amount("widget") - 2.0).abs() < 1e-9);
        assert!((pool.amount("water") - 980.0).abs() < 1e-9);
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
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
            line: None,
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
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
            line: None,
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

    #[test]
    fn line_results_outputs_deposited_matches_the_post_multiplier_amount() {
        // Issue #317: depletion consumes `outputs_deposited`, which must be
        // the *actual* stockpile deposit — including productivity_multiplier
        // — not `recipe.outputs[].quantity * scale`, which silently omits it.
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
            0.75,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
        );

        let smelter = outcome
            .building_results
            .iter()
            .find(|b| b.building_type == "smelter")
            .expect("smelter ran");
        let line = smelter
            .line_results
            .first()
            .expect("smelter has a line result");
        let (commodity, deposited) = line
            .outputs_deposited
            .iter()
            .find(|(id, _)| id == "iron_plate")
            .expect("iron_plate output recorded");

        assert_eq!(commodity, "iron_plate");
        assert!(
            (deposited - 0.75).abs() < 1e-6,
            "expected outputs_deposited to carry the 0.75x productivity multiplier, got {deposited}"
        );
        // Sanity: the recorded figure matches what actually landed in the pool.
        assert!((pool.amount("iron_plate") - *deposited).abs() < 1e-9);
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
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
            line: None,
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
            line: None,
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
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
            line: None,
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
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
            line: None,
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

    /// A prospected deposit must still be strictly better than bare ground.
    ///
    /// The trace band added in #317 is the whole reason this needs pinning: if
    /// `TRACE_DEPOSIT_RATIO` were ever tuned up to or past the 0.5 deposit floor,
    /// prospecting would stop paying for itself and the mechanic would be dead
    /// content, with nothing else in the suite noticing.
    #[test]
    fn a_real_deposit_outproduces_bare_ground() {
        let reg = make_registry_with_vein_mine();
        let placed = buildings(&["structural_mine"]);

        let run = |deposits: std::collections::HashMap<String, f32>| {
            let mut pool = ColonyPool::new();
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
                Some(&deposits),
                &crate::modifier::ModifierAccumulator::new(),
                &crate::modifier::DifficultyScalar::new(),
            );
            pool.amount("structural_ore")
        };

        let mut bare = std::collections::HashMap::new();
        bare.insert("conductive_ore".to_string(), 0.8_f32);
        let mut leanest_real = std::collections::HashMap::new();
        leanest_real.insert("structural_ore".to_string(), 0.0001_f32);

        let bare_yield = run(bare);
        let deposit_yield = run(leanest_real);
        assert!(
            bare_yield > 0.0,
            "bare ground must yield a trickle, not nothing"
        );
        assert!(
            deposit_yield > bare_yield,
            "even the leanest real deposit ({deposit_yield}) must beat bare ground \
             ({bare_yield}), or prospecting is pointless"
        );
    }

    #[test]
    fn deposit_gating_throttles_output_to_trace_with_no_matching_deposit() {
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

        // Bare ground still yields a trickle (#317): prospecting is optional in
        // the early game, not a hard prerequisite for extraction.
        let mine = &outcome.building_results[0];
        assert!(
            (mine.scale - TRACE_DEPOSIT_RATIO).abs() < 1e-9,
            "expected trace scale {TRACE_DEPOSIT_RATIO}, got {}",
            mine.scale
        );
        assert!(mine.shortfalls.iter().any(|s| matches!(
            &s.reason,
            ShortfallReason::DepositShort { commodity_id } if commodity_id == "structural_ore"
        )));
        assert!((pool.amount("structural_ore") - 10.0 * TRACE_DEPOSIT_RATIO).abs() < 1e-9);
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
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
            line: None,
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
            line: None,
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
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
            line: None,
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
            line: None,
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
            line: None,
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
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
            line: None,
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

    /// A multi-line building's summary must cover *every* line. Before #272
    /// this function resolved a single "pick-one" recipe, which for the shipped
    /// `fabrication_complex` shape silently dropped the machine shop — the
    /// colony panel would show the foundry alone and never mention that metal
    /// is being consumed into components every sol.
    #[test]
    fn io_summary_covers_every_line_not_just_the_first() {
        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "complex".into(),
            name: "complex".into(),
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
        });
        let recipe = |id: &str, line: &str, inputs: Vec<(&str, f64)>, outputs: Vec<(&str, f64)>| {
            let ing = |v: Vec<(&str, f64)>| {
                v.into_iter()
                    .map(|(id, quantity)| Ingredient {
                        id: id.into(),
                        quantity,
                    })
                    .collect::<Vec<_>>()
            };
            RecipeDef {
                id: id.into(),
                name: id.into(),
                building: "complex".into(),
                cycle_sols: 1,
                inputs: ing(inputs),
                outputs: ing(outputs),
                concurrent: false,
                line: Some(line.into()),
                power_draw: 0.0,
            }
        };
        // Named so the machine shop sorts *after* the foundry — the exact shape
        // that made the old lexicographically-first resolution hide it.
        reg.insert_recipe(recipe(
            "a_smelt",
            "foundry",
            vec![("ore", 5.0)],
            vec![("metal", 2.5)],
        ));
        reg.insert_recipe(recipe(
            "z_machine",
            "machine_shop",
            vec![("metal", 2.0)],
            vec![("components", 1.0)],
        ));

        let summary = building_io_summary("complex", &std::collections::HashMap::new(), &reg);

        assert!(
            summary.recipe_ids.contains(&"z_machine".to_string()),
            "the machine shop line was dropped from the summary: {:?}",
            summary.recipe_ids
        );
        let outputs: std::collections::HashMap<&str, f64> = summary
            .outputs
            .iter()
            .map(|(id, q)| (id.as_str(), *q))
            .collect();
        assert_eq!(outputs.get("components"), Some(&1.0));
        assert_eq!(outputs.get("metal"), Some(&2.5));
        // `metal` is both produced by one line and consumed by the other, and
        // must appear on both sides rather than being netted away.
        let inputs: std::collections::HashMap<&str, f64> = summary
            .inputs
            .iter()
            .map(|(id, q)| (id.as_str(), *q))
            .collect();
        assert_eq!(inputs.get("metal"), Some(&2.0));
        assert_eq!(inputs.get("ore"), Some(&5.0));

        // And it must still agree with what production actually runs.
        let mut pool = ColonyPool::new();
        pool.deposit("ore", 100.0);
        pool.deposit("metal", 100.0);
        let outcome = process_production(&mut pool, &[("complex".to_string(), 1)], 100.0, &reg);
        let result = outcome
            .building_results
            .iter()
            .find(|r| r.building_type == "complex")
            .expect("complex should have produced");
        let mut ran: Vec<String> = result
            .line_results
            .iter()
            .map(|l| l.recipe_id.clone())
            .collect();
        ran.sort();
        let mut summarised = summary.recipe_ids.clone();
        summarised.sort();
        assert_eq!(
            ran, summarised,
            "the summary and the production step must agree on which recipes run"
        );
    }

    // ── Production lines (issue #272) ─────────────────────────────────────

    fn lines_registry() -> ContentRegistry {
        let mut reg = ContentRegistry::default();
        let b = |id: &str| BuildingDef {
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
        };
        reg.insert_building(b("complex"));
        reg.insert_building(b("legacy"));

        let r = |id: &str, bld: &str, con: bool, line: Option<&str>| RecipeDef {
            id: id.into(),
            name: id.into(),
            building: bld.into(),
            cycle_sols: 1,
            inputs: vec![],
            outputs: vec![],
            concurrent: con,
            line: line.map(Into::into),
            power_draw: 0.0,
        };

        // Two independent switchable lines on one building — the thing that was
        // impossible before #272.
        reg.insert_recipe(r("smelt_a", "complex", false, Some("smelting")));
        reg.insert_recipe(r("smelt_b", "complex", false, Some("smelting")));
        reg.insert_recipe(r("mach_p", "complex", false, Some("machining")));
        reg.insert_recipe(r("mach_q", "complex", false, Some("machining")));
        // Plus an always-on line alongside them.
        reg.insert_recipe(r("vent", "complex", true, None));

        // A pre-#272 shaped building: two unlined alternatives + one concurrent.
        reg.insert_recipe(r("old_x", "legacy", false, None));
        reg.insert_recipe(r("old_y", "legacy", false, None));
        reg.insert_recipe(r("old_always", "legacy", true, None));
        reg
    }

    /// The core new capability: two switchable lines coexist, each with its own
    /// selection, and both run.
    #[test]
    fn two_named_lines_each_keep_their_own_selection() {
        let reg = lines_registry();
        let mut active = std::collections::HashMap::new();
        active.insert(
            line_selection_key("complex", Some("smelting")),
            "smelt_b".to_string(),
        );
        active.insert(
            line_selection_key("complex", Some("machining")),
            "mach_q".to_string(),
        );

        let lines = lines_for_building("complex", &active, &reg);
        let running: Vec<&str> = lines.iter().map(|l| l.selected.id.as_str()).collect();

        assert!(
            running.contains(&"smelt_b"),
            "smelting selection lost: {running:?}"
        );
        assert!(
            running.contains(&"mach_q"),
            "machining selection lost: {running:?}"
        );
        assert!(
            running.contains(&"vent"),
            "always-on line missing: {running:?}"
        );
        assert_eq!(running.len(), 3, "one per line: {running:?}");
    }

    /// Selecting on one line must not disturb another — this is exactly what the
    /// old single-key-per-building map got wrong.
    #[test]
    fn selecting_on_one_line_leaves_the_other_line_alone() {
        let reg = lines_registry();
        let mut active = std::collections::HashMap::new();
        active.insert(
            line_selection_key("complex", Some("smelting")),
            "smelt_b".to_string(),
        );

        let lines = lines_for_building("complex", &active, &reg);
        let pick = |line: &str| {
            lines
                .iter()
                .find(|l| l.line.as_deref() == Some(line))
                .map(|l| l.selected.id.clone())
        };
        assert_eq!(
            pick("smelting").as_deref(),
            Some("smelt_b"),
            "explicit choice"
        );
        assert_eq!(
            pick("machining").as_deref(),
            Some("mach_p"),
            "untouched line falls back to its own deterministic default, not to nothing"
        );
    }

    /// A pre-#272 building keeps its exact previous shape: one pick-one choice
    /// plus the always-on recipe, and the default-line selection is still keyed
    /// on the bare building id so old saves resolve unchanged.
    #[test]
    fn an_unlined_building_behaves_exactly_as_before() {
        let reg = lines_registry();

        // No selection: deterministic default is the smallest id.
        let empty: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let lines = lines_for_building("legacy", &empty, &reg);
        let running: Vec<&str> = lines.iter().map(|l| l.selected.id.as_str()).collect();
        assert!(
            running.contains(&"old_x"),
            "default should be old_x: {running:?}"
        );
        assert!(
            !running.contains(&"old_y"),
            "alternatives must not both run"
        );
        assert!(
            running.contains(&"old_always"),
            "concurrent recipe still always runs"
        );

        // A pre-#272 save keys the selection on the bare building id.
        let mut legacy_save = std::collections::HashMap::new();
        legacy_save.insert("legacy".to_string(), "old_y".to_string());
        let lines = lines_for_building("legacy", &legacy_save, &reg);
        let running: Vec<&str> = lines.iter().map(|l| l.selected.id.as_str()).collect();
        assert!(
            running.contains(&"old_y"),
            "a pre-#272 selection must still resolve: {running:?}"
        );
        assert_eq!(
            line_selection_key("legacy", None),
            "legacy",
            "the default line's key is the bare building id — this is the whole \
             reason no save migration is needed"
        );
    }

    /// A selection naming a recipe on a *different* line is ignored rather than
    /// silently running nothing.
    #[test]
    fn a_selection_from_the_wrong_line_falls_back_to_that_lines_default() {
        let reg = lines_registry();
        let mut active = std::collections::HashMap::new();
        // Point the smelting line at a machining recipe.
        active.insert(
            line_selection_key("complex", Some("smelting")),
            "mach_q".to_string(),
        );

        let lines = lines_for_building("complex", &active, &reg);
        let smelting = lines
            .iter()
            .find(|l| l.line.as_deref() == Some("smelting"))
            .expect("smelting line should still exist");
        assert_eq!(
            smelting.selected.id, "smelt_a",
            "a cross-line id must fall back to the line's own default"
        );
    }

    /// Line partitioning must be stable — `ContentRegistry` iterates a `HashMap`.
    #[test]
    fn line_order_and_alternatives_are_deterministic() {
        let reg = lines_registry();
        let empty: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let first: Vec<Option<String>> = lines_for_building("complex", &empty, &reg)
            .iter()
            .map(|l| l.line.clone())
            .collect();
        for _ in 0..12 {
            let again: Vec<Option<String>> = lines_for_building("complex", &empty, &reg)
                .iter()
                .map(|l| l.line.clone())
                .collect();
            assert_eq!(first, again, "line order must not vary between calls");
        }
        let lines = lines_for_building("complex", &empty, &reg);
        let smelting = lines
            .iter()
            .find(|l| l.line.as_deref() == Some("smelting"))
            .unwrap();
        assert_eq!(
            smelting
                .alternatives
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            vec!["smelt_a", "smelt_b"],
            "alternatives are id-sorted so a picker's order is stable"
        );
        assert!(!smelting.always_on);
        let vent = lines.iter().find(|l| l.always_on).unwrap();
        assert_eq!(
            vent.alternatives.len(),
            1,
            "an always-on line has nothing to choose"
        );
    }

    /// A building with no recipes yields no lines — not a panic, not a phantom.
    #[test]
    fn a_building_with_no_recipes_has_no_lines() {
        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "silo".into(),
            name: "silo".into(),
            description: String::new(),
            category: BuildingCategory::Storage,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
        });
        let empty: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        assert!(lines_for_building("silo", &empty, &reg).is_empty());
    }

    // ── Lines running live in the pipeline (issue #272) ───────────────────

    /// Registry with a fabrication complex running two independent lines that
    /// consume *different* feedstocks, so starving one cannot excuse the other.
    fn live_lines_registry() -> ContentRegistry {
        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "complex".into(),
            name: "Fabrication Complex".into(),
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
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            output_scaling: None,
            site_requirements: Vec::new(),
        });
        let r = |id: &str, line: Option<&str>, con: bool, i: &[(&str, f64)], o: &[(&str, f64)]| {
            RecipeDef {
                id: id.into(),
                name: id.into(),
                building: "complex".into(),
                cycle_sols: 1,
                inputs: i
                    .iter()
                    .map(|(id, q)| Ingredient {
                        id: (*id).into(),
                        quantity: *q,
                    })
                    .collect(),
                outputs: o
                    .iter()
                    .map(|(id, q)| Ingredient {
                        id: (*id).into(),
                        quantity: *q,
                    })
                    .collect(),
                concurrent: con,
                line: line.map(Into::into),
                power_draw: 0.0,
            }
        };
        reg.insert_recipe(r(
            "smelt",
            Some("smelting"),
            false,
            &[("ore", 10.0)],
            &[("metal", 5.0)],
        ));
        reg.insert_recipe(r(
            "machine",
            Some("machining"),
            false,
            &[("chem", 10.0)],
            &[("part", 5.0)],
        ));
        reg
    }

    fn run_lines(pool: &mut ColonyPool, reg: &ContentRegistry) -> ProductionStepOutcome {
        process_production(pool, &[("complex".to_string(), 1)], 1000.0, reg)
    }

    /// The capability, end to end: both lines run in the same turn and each
    /// produces from its own feedstock.
    #[test]
    fn two_lines_both_produce_in_one_turn() {
        let reg = live_lines_registry();
        let mut pool = ColonyPool::new();
        pool.deposit("ore", 100.0);
        pool.deposit("chem", 100.0);

        run_lines(&mut pool, &reg);

        assert!(
            (pool.amount("metal") - 5.0).abs() < 1e-9,
            "metal={}",
            pool.amount("metal")
        );
        assert!(
            (pool.amount("part") - 5.0).abs() < 1e-9,
            "part={}",
            pool.amount("part")
        );
        assert!((pool.amount("ore") - 90.0).abs() < 1e-9);
        assert!((pool.amount("chem") - 90.0).abs() < 1e-9);
    }

    /// Independent throttling — the design decision this was built around. One
    /// line starved of its feedstock must not drag the other down.
    #[test]
    fn starving_one_line_leaves_the_other_at_full_rate() {
        let reg = live_lines_registry();
        let mut pool = ColonyPool::new();
        pool.deposit("ore", 2.0); // 20% of what the smelting line wants
        pool.deposit("chem", 100.0); // machining is fully fed

        let outcome = run_lines(&mut pool, &reg);

        // Machining ran flat out despite smelting starving.
        assert!(
            (pool.amount("part") - 5.0).abs() < 1e-9,
            "machining should be unaffected, part={}",
            pool.amount("part")
        );
        // Smelting ran at 20%.
        assert!(
            (pool.amount("metal") - 1.0).abs() < 1e-9,
            "smelting should be throttled to 20%, metal={}",
            pool.amount("metal")
        );

        let result = outcome
            .building_results
            .iter()
            .find(|r| r.building_type == "complex")
            .expect("complex should have produced");
        let line_scale = |name: &str| {
            result
                .line_results
                .iter()
                .find(|l| l.line.as_deref() == Some(name))
                .map(|l| l.scale)
                .expect("line result present")
        };
        assert!((line_scale("machining") - 1.0).abs() < 1e-9);
        assert!((line_scale("smelting") - 0.2).abs() < 1e-9);
        // The headline scale summarises the worst line, so "is anything wrong
        // here?" still answers yes.
        assert!(
            (result.scale - 0.2).abs() < 1e-9,
            "headline scale={}",
            result.scale
        );
    }

    /// Under the pre-#272 shared-scale model this test's `part` would be 1.0
    /// (dragged to 20% by the ore shortage). Pinning the *difference* keeps the
    /// independence from silently regressing to pooled behaviour.
    #[test]
    fn independence_is_what_distinguishes_lines_from_the_old_shared_scale() {
        let reg = live_lines_registry();
        let mut pool = ColonyPool::new();
        pool.deposit("ore", 2.0);
        pool.deposit("chem", 100.0);
        run_lines(&mut pool, &reg);

        let shared_scale_would_give = 5.0 * 0.2;
        assert!(
            (pool.amount("part") - shared_scale_would_give).abs() > 1e-6,
            "part={} matches the old pooled-scale answer; independence has regressed",
            pool.amount("part")
        );
    }

    /// Player selection on one line changes only that line's output.
    #[test]
    fn switching_one_lines_recipe_leaves_the_other_running() {
        let mut reg = live_lines_registry();
        reg.insert_recipe(RecipeDef {
            id: "smelt_alt".into(),
            name: "smelt_alt".into(),
            building: "complex".into(),
            cycle_sols: 1,
            inputs: vec![Ingredient {
                id: "ore".into(),
                quantity: 10.0,
            }],
            outputs: vec![Ingredient {
                id: "alloy".into(),
                quantity: 5.0,
            }],
            concurrent: false,
            line: Some("smelting".into()),
            power_draw: 0.0,
        });

        let mut pool = ColonyPool::new();
        pool.deposit("ore", 100.0);
        pool.deposit("chem", 100.0);

        let mut active = std::collections::HashMap::new();
        active.insert(
            line_selection_key("complex", Some("smelting")),
            "smelt_alt".to_string(),
        );
        let mut resources = ColonyResourcePool::new();
        super::process_production_scaled(
            &mut ColonyStores::new(&mut pool, &mut resources, &reg),
            &ProductionInput::from_types(&[("complex".to_string(), 1)]),
            1000.0,
            &reg,
            1.0,
            0.0, // power_import (issue #383)
            1.0,
            false,
            1.0,
            &active,
            &[],
            None,
            &crate::modifier::ModifierAccumulator::default(),
            &crate::modifier::DifficultyScalar::default(),
            None,
        );

        assert!(
            (pool.amount("alloy") - 5.0).abs() < 1e-9,
            "the switched line ran"
        );
        assert_eq!(pool.amount("metal"), 0.0, "the replaced recipe did not run");
        assert!(
            (pool.amount("part") - 5.0).abs() < 1e-9,
            "the other line is untouched by the switch"
        );
    }

    // ── Site output scaling (issue #411) ────────────────────────────────────

    /// A generator whose output and capacity both ride on a site multiplier.
    fn site_scaled_generator_registry() -> ContentRegistry {
        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "array".into(),
            name: "Array".into(),
            description: String::new(),
            category: BuildingCategory::Power,
            construction_cost: vec![],
            power_delta: -20.0,
            worker_slots: 0,
            labor_required: 0,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            site_requirements: Vec::new(),
            output_scaling: None,
        });
        reg.insert_recipe(RecipeDef {
            id: "generate".into(),
            name: "Generate".into(),
            building: "array".into(),
            cycle_sols: 1,
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "power".into(),
                quantity: 24.0,
            }],
            power_draw: 0.0,
            concurrent: false,
            line: None,
        });
        reg
    }

    /// Run one sol with `array` at the given site multiplier, returning the
    /// power produced and the grid capacity it contributed.
    fn run_scaled(multiplier: Option<f64>) -> (f64, f64) {
        let reg = site_scaled_generator_registry();
        let mults: std::collections::HashMap<String, f64> = multiplier
            .map(|m| std::iter::once(("array".to_string(), m)).collect())
            .unwrap_or_default();
        let site = multiplier.map(|_| &mults);

        let buildings = ProductionInput::from_types(&[("array".to_string(), 1)]);
        let grid = compute_power_grid_scaled(
            &buildings,
            &reg,
            1.0,
            &std::collections::HashMap::new(),
            site,
        );

        let mut pool = ColonyPool::new();
        let mut resources = ColonyResourcePool::new();
        super::process_production_scaled(
            &mut ColonyStores::new(&mut pool, &mut resources, &reg),
            &buildings,
            0.0,
            &reg,
            1.0,
            0.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
            None,
            &crate::modifier::ModifierAccumulator::new(),
            &crate::modifier::DifficultyScalar::new(),
            site,
        );
        // `power` is unregistered here, so it lands in the tradeable pool
        // rather than the colony-local resource store — which is all this
        // test needs: the question is whether the multiplier was applied.
        (pool.amount("power"), grid.capacity)
    }

    #[test]
    fn a_site_multiplier_scales_recipe_output() {
        let (full, _) = run_scaled(Some(1.0));
        let (half, _) = run_scaled(Some(0.5));
        assert!((full - 24.0).abs() < 1e-6, "got {full}");
        assert!((half - 12.0).abs() < 1e-6, "got {half}");
    }

    #[test]
    fn a_site_multiplier_scales_grid_capacity_by_the_same_factor() {
        // The two must move together, or a generator advertises headroom its
        // own output cannot fill (or fills headroom it never supplied).
        let (full_out, full_cap) = run_scaled(Some(1.0));
        let (half_out, half_cap) = run_scaled(Some(0.5));
        assert!((full_cap - 20.0).abs() < 1e-6, "got {full_cap}");
        assert!((half_cap - 10.0).abs() < 1e-6, "got {half_cap}");
        assert!(
            ((half_out / full_out) - (half_cap / full_cap)).abs() < 1e-9,
            "output and capacity scaled by different factors"
        );
    }

    #[test]
    fn a_multiplier_above_one_raises_both() {
        let (out, cap) = run_scaled(Some(1.5));
        assert!((out - 36.0).abs() < 1e-6, "got {out}");
        assert!((cap - 30.0).abs() < 1e-6, "got {cap}");
    }

    #[test]
    fn no_site_data_leaves_a_building_exactly_as_it_was() {
        let (out, cap) = run_scaled(None);
        assert!((out - 24.0).abs() < 1e-6, "got {out}");
        assert!((cap - 20.0).abs() < 1e-6, "got {cap}");
    }
}
