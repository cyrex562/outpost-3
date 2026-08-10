//! Authored content types: the serde-deserialised shapes for commodities,
//! recipes, and buildings as they appear in YAML pack files.

use serde::{Deserialize, Serialize};

/// Pack-level manifest (`pack.yaml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackManifest {
    /// Unique slug for this pack (e.g. `"base-colony"`).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Optional description shown in pack-selection UI.
    #[serde(default)]
    pub description: String,
}

/// Physical phase of a commodity (solid, liquid, gas, plasma).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Solid material.
    Solid,
    /// Liquid material.
    Liquid,
    /// Gaseous material.
    Gas,
    /// High-energy plasma (rare; requires special containment).
    Plasma,
}

/// Production tier: distinguishes survival basics from advanced growth chains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CommodityTier {
    /// Cheap, soloable; required for colony survival.
    #[default]
    Basic,
    /// Competes for slots and labour; drives growth chains.
    Advanced,
}

/// A tradeable / produceable commodity authored in a content pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommodityDef {
    /// Unique identifier across all packs.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Short description shown to players.
    #[serde(default)]
    pub description: String,
    /// Logical category tag (e.g. `"metallic_ore"`, `"food"`, `"fuel"`).
    pub category: String,
    /// Physical storage phase; influences warehouse slot types.
    #[serde(default = "Phase::solid_default")]
    pub phase: Phase,
    /// Base trade value in standard credits.
    pub base_value: f64,
    /// Whether this commodity can be bought/sold on the market.
    #[serde(default = "bool_true")]
    pub tradeable: bool,
    /// Production tier: basic (survival) or advanced (growth).
    #[serde(default)]
    pub tier: CommodityTier,
    /// Storage weight per unit (arbitrary units; used for capacity planning).
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 {
    1.0
}

/// How a [`ResourceDef`] behaves over time.
///
/// Both kinds are cleared at the end of every colony sol — a colony resource
/// never carries over. The distinction is what the number *means* to a reader:
/// a `Flow` is throughput that turn, a `Capacity` is a standing capability the
/// colony's buildings re-establish each turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    /// Produced and consumed within a single sol; any surplus is lost.
    #[default]
    Flow,
    /// A standing capability measured against demand rather than drawn down
    /// (housing slots vs population). Re-established each sol by its buildings.
    Capacity,
}

/// A colony-local resource authored in a content pack (issue #304).
///
/// Distinct from [`CommodityDef`] because these are **not tradeable**: they are
/// produced and consumed in place and never enter a hauler, a trade route, or
/// the market. That separation is structural — resources live in
/// [`crate::colony::ColonyResourcePool`], and the trade pipeline is only ever
/// handed the commodity pool, so there is no flag for a caller to forget to
/// check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDef {
    /// Unique identifier, in the same namespace as commodity ids.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Short description shown to players.
    #[serde(default)]
    pub description: String,
    /// Whether this is per-sol throughput or a standing capacity.
    #[serde(default)]
    pub kind: ResourceKind,
    /// Unit label for display (e.g. `"MW"`, `"slots"`, `"RP"`).
    #[serde(default)]
    pub unit: String,
}

impl Phase {
    fn solid_default() -> Self {
        Phase::Solid
    }
}

fn bool_true() -> bool {
    true
}

/// One ingredient in a recipe (commodity id + quantity).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ingredient {
    /// Commodity id.
    pub id: String,
    /// Amount consumed (or produced) per production cycle.
    pub quantity: f64,
}

/// A production recipe: consumes inputs, produces outputs, requires a building.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeDef {
    /// Unique identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Id of the building that can execute this recipe.
    pub building: String,
    /// List of consumed commodities per cycle.
    pub inputs: Vec<Ingredient>,
    /// List of produced commodities per cycle.
    pub outputs: Vec<Ingredient>,
    /// Duration in colony-sols per production cycle.
    #[serde(default = "default_cycle_sols")]
    pub cycle_sols: u32,
    /// Power consumed per cycle (kW); zero means no power requirement.
    #[serde(default)]
    pub power_draw: f64,
    /// When `true`, this recipe always runs alongside every other recipe
    /// marked `concurrent` for the same `building`, every turn — it never
    /// participates in [`crate::colony::Colony::active_recipes`]'s
    /// pick-one selection (issue: "true simultaneous multi-output
    /// buildings" — playtest feedback's deferred multi-function starter
    /// building idea). A building can mix at most one pick-one recipe set
    /// (the ordinary, `active_recipes`-selected kind) with any number of
    /// always-on `concurrent` recipes; see `production.rs`'s module doc
    /// comment for how the two combine into one shared per-instance scale
    /// factor. Defaults to `false` (ordinary pick-one-alternative recipe),
    /// so every recipe authored before this field existed is unaffected.
    #[serde(default)]
    pub concurrent: bool,
    /// Production line this recipe belongs to within its building (issue #272).
    ///
    /// Recipes sharing a line on the same building are **alternatives** — one of
    /// them runs. Different lines run **simultaneously and throttle
    /// independently**, which is what lets one building host several genuinely
    /// separate production chains rather than one switchable chain plus a set of
    /// always-on extras.
    ///
    /// `None` (the default) means the building's *default* line, so every recipe
    /// authored before this field existed keeps its exact previous behaviour:
    /// non-`concurrent` recipes with no line are alternatives to each other, and
    /// each [`Self::concurrent`] recipe becomes a line of its own containing only
    /// itself — a line with one member has nothing to choose, so it always runs.
    ///
    /// Setting both `line` and `concurrent` is redundant rather than an error:
    /// `line` wins, and the recipe is an alternative within that line like any
    /// other. `ContentRegistry::lint` flags it.
    #[serde(default)]
    pub line: Option<String>,
}

fn default_cycle_sols() -> u32 {
    1
}

/// Category of a building (production, storage, housing, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingCategory {
    /// Produces commodities from inputs. Legacy/generic — prefer
    /// [`BuildingCategory::Extraction`] or [`BuildingCategory::Processing`]
    /// for new buildings (issue #166).
    Production,
    /// Stores commodities.
    Storage,
    /// Houses population.
    Housing,
    /// Produces power.
    Power,
    /// Research facility.
    Research,
    /// Pulls raw resources from the environment (mines, wells, farms) with
    /// little or no commodity input (issue #166).
    Extraction,
    /// Refines or manufactures raw resources into higher-value commodities
    /// (issue #166).
    Processing,
    /// Atmosphere, water, and other habitat-sustaining infrastructure
    /// (issue #166).
    LifeSupport,
    /// Colony/system defense and security (issue #166).
    Defense,
    /// Administrative, commercial, and other non-production colony services
    /// (issue #166).
    Services,
    /// Site preparation that expands what the colony can physically host —
    /// graded pads, roads, connecting tubes, buried utility runs (issue #306).
    ///
    /// Projects in this category typically complete into
    /// [`BuildingDef::grants_slot_capacity`] rather than into a standing
    /// building.
    Infrastructure,
    /// Hex-contamination cleanup (issue #388).
    ///
    /// Projects in this category typically complete into
    /// [`BuildingDef::contamination_reduction`] rather than into a standing
    /// building — the same "project, not a building" shape
    /// [`Self::Infrastructure`] gives site preparation.
    Remediation,
    /// Generic / uncategorised.
    Other,
}

/// One thing a site must be like before a building can be constructed there
/// (issue #410).
///
/// Authored per building rather than inferred, the same way
/// [`BuildingDef::max_instances`] is: which structures care about where they
/// stand is a content decision, not an engine one.
///
/// Evaluation lives in [`crate::site`], which is where the map and system
/// types needed to answer these questions are reachable.
///
/// # Radius
///
/// Hex-scoped variants carry `within_hexes`, defaulting to `0` — the colony's
/// own hex. A larger radius means "somewhere near enough to exploit", which is
/// what makes e.g. a coastal power plant buildable on the land hex beside the
/// water rather than requiring the colony to sit *on* an unbuildable ocean
/// tile.
// No `Eq`: `MinGeothermalGradient` carries an f32.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SiteCondition {
    /// At least one hex within range has one of these terrains.
    Terrain {
        /// Any one of these satisfies the requirement.
        any_of: Vec<crate::map::Terrain>,
        /// Search radius in hexes; `0` means the colony's own hex.
        #[serde(default)]
        within_hexes: u32,
    },
    /// At least one hex within range holds a deposit of this commodity.
    Deposit {
        /// Content-pack commodity id, matching [`crate::map::Deposit::commodity_id`].
        commodity: String,
        /// Search radius in hexes; `0` means the colony's own hex.
        #[serde(default)]
        within_hexes: u32,
    },
    /// The body's atmosphere is at least this dense.
    ///
    /// Body-scoped rather than hex-scoped, so unlike the two above it can be
    /// evaluated for an outpost, which is anchored to a body without a surface
    /// hex.
    MinAtmosphere {
        /// The thinnest atmosphere that satisfies this.
        density: crate::system::AtmosphereDensity,
    },
    /// The site's geothermal gradient is at least this high (issue #412).
    ///
    /// Reads [`crate::map::HexCell::geothermal_gradient`] — how shallow magma
    /// sits. Paired with [`SiteRequirement::waived_by_tech`], this is what
    /// expresses "you cannot reach usable heat here without drilling
    /// technology."
    MinGeothermalGradient {
        /// The shallowest magma that satisfies this, in `[0.0, 1.0]`.
        min: f32,
    },
}

impl SiteCondition {
    /// A short human-readable statement of the condition, for error messages
    /// and the build UI. Phrased as the requirement, not as a failure, so the
    /// same string serves a met and an unmet row.
    #[must_use]
    pub fn describe(&self) -> String {
        fn within(radius: u32) -> String {
            match radius {
                0 => "on this site".to_string(),
                1 => "within 1 hex".to_string(),
                n => format!("within {n} hexes"),
            }
        }
        match self {
            Self::Terrain {
                any_of,
                within_hexes,
            } => {
                let names: Vec<String> = any_of
                    .iter()
                    .map(|t| format!("{t:?}").to_lowercase())
                    .collect();
                format!("{} {}", names.join(" or "), within(*within_hexes))
            }
            Self::Deposit {
                commodity,
                within_hexes,
            } => {
                format!("{commodity} deposit {}", within(*within_hexes))
            }
            Self::MinAtmosphere { density } => {
                format!(
                    "{} atmosphere or denser",
                    format!("{density:?}").to_lowercase()
                )
            }
            Self::MinGeothermalGradient { min } => {
                format!("geothermal gradient of at least {:.0}%", min * 100.0)
            }
        }
    }
}

/// A site condition, optionally overcome by a technology (issue #414).
///
/// The wrapper exists because some conditions are not absolute: a site too
/// cold for geothermal heat is unreachable *until you can drill deep enough*,
/// and then it is not. That is a **conditional tech gate** — it depends on the
/// site, not just the building — which [`BuildingDef::tech_prerequisite`]
/// cannot express, since that gates a building everywhere at once.
///
/// Kept general rather than special-cased to geothermal: any condition can be
/// waived, so a future pressurisation tech could lift an atmosphere
/// requirement without this type changing again.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteRequirement {
    /// What the site must be like.
    #[serde(flatten)]
    pub condition: SiteCondition,
    /// A researched technology that satisfies this requirement regardless of
    /// the site. `None` — the default — means the condition is absolute.
    #[serde(default)]
    pub waived_by_tech: Option<String>,
}

impl SiteRequirement {
    /// An absolute requirement — no technology overcomes it.
    #[must_use]
    pub fn new(condition: SiteCondition) -> Self {
        Self {
            condition,
            waived_by_tech: None,
        }
    }

    /// A requirement a researched technology lifts.
    #[must_use]
    pub fn waivable(condition: SiteCondition, tech: impl Into<String>) -> Self {
        Self {
            condition,
            waived_by_tech: Some(tech.into()),
        }
    }

    /// A short human-readable statement, naming the waiving tech when there is
    /// one — "X, or Y" reads as a genuine choice, where the bare condition
    /// would look like a wall.
    #[must_use]
    pub fn describe(&self) -> String {
        match &self.waived_by_tech {
            Some(tech) => format!("{} (or {tech})", self.condition.describe()),
            None => self.condition.describe(),
        }
    }
}

/// A site property a building's output can be scaled by (issue #411).
///
/// Every variant resolves to a normalised `[0.0, 1.0]` reading, so the
/// multiplier curve in [`SiteScaling`] is the same shape whatever drives it —
/// which is what lets a new property (a geothermal gradient, insolation) be
/// added without touching the scaling mechanism.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "property", rename_all = "snake_case")]
pub enum SiteProperty {
    /// Richness of a named commodity's deposit at the site.
    ///
    /// Distinct from #232's deposit *gating*, which throttles a recipe that
    /// consumes a vein commodity. This reads the same richness as a plain
    /// input to an output curve, for a building whose yield depends on what
    /// is under it without consuming it.
    DepositRichness {
        /// Content-pack commodity id.
        commodity: String,
    },
    /// The body's atmosphere density, `Vacuum` = 0 through `Dense` = 1.
    AtmosphereDensity,
    /// The site hex's elevation, already stored normalised.
    Elevation,
    /// The site hex's geothermal gradient (issue #412) — how shallow magma
    /// sits beneath it.
    GeothermalGradient,
}

/// How a building's output responds to a site property (issue #411).
///
/// Linear between two authored endpoints: `at_min` is the multiplier where the
/// property reads `0.0`, `at_max` where it reads `1.0`. Two endpoints rather
/// than a named curve because every case wanted so far is monotonic, and an
/// author reading `at_min: 0.2, at_max: 1.5` can see the whole behaviour
/// without knowing what "linear" or "quadratic" would have meant here.
///
/// `at_min` above `at_max` is allowed and means the output falls as the
/// property rises — nothing assumes the relationship is positive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteScaling {
    /// What drives the multiplier.
    #[serde(flatten)]
    pub property: SiteProperty,
    /// Multiplier where the property reads `0.0`.
    pub at_min: f64,
    /// Multiplier where the property reads `1.0`.
    pub at_max: f64,
}

impl SiteScaling {
    /// The multiplier for a normalised property reading.
    ///
    /// `reading` is clamped to `[0.0, 1.0]` first: a property implementation
    /// returning something out of range is a bug, but silently extrapolating
    /// the curve past its authored endpoints would turn that bug into a
    /// wildly wrong yield rather than a merely capped one.
    #[must_use]
    pub fn multiplier_at(&self, reading: f64) -> f64 {
        let t = reading.clamp(0.0, 1.0);
        (self.at_min + (self.at_max - self.at_min) * t).max(0.0)
    }
}

/// A structure that can be constructed in a colony.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingDef {
    /// Unique identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Short description.
    #[serde(default)]
    pub description: String,
    /// Logical category.
    pub category: BuildingCategory,
    /// Construction cost list (commodity id → quantity).
    #[serde(default)]
    pub construction_cost: Vec<Ingredient>,
    /// Power consumed per sol (negative = produced).
    #[serde(default)]
    pub power_delta: f64,
    /// Worker slots required to operate.
    #[serde(default)]
    pub worker_slots: u32,
    /// Labor units consumed from the colony pool each construction turn.
    #[serde(default = "default_labor")]
    pub labor_required: u32,
    /// Number of build slots this building occupies.
    #[serde(default = "default_slot_cost")]
    pub slot_cost: u32,
    /// Number of colony-sol turns required to complete construction.
    #[serde(default = "default_construction_turns")]
    pub construction_turns: u32,
    /// Optional tech node that must be researched before this building can be queued.
    #[serde(default)]
    pub tech_prerequisite: Option<String>,
    /// Per-sol upkeep drained from the colony pool while the building is
    /// operational (issue #180).
    ///
    /// Multiplied by the `MaintenanceConsumption` difficulty scalar and short-
    /// circuited by `GameState::maintenance_enabled == false`.  Empty (the
    /// default) means the building has no maintenance requirement.
    ///
    /// Under-construction projects do **not** pay maintenance; the drain begins
    /// the sol after `BuildingConstructed` fires.
    #[serde(default)]
    pub maintenance: Vec<Ingredient>,
    /// Default staffing priority for instances of this building (issue #307).
    ///
    /// `1` is staffed first, [`MAX_BUILDING_PRIORITY`] last. Labour is allocated
    /// in priority order, so a lower number means "keep this running when
    /// workers are short" — author life support and food production ahead of
    /// industry, and storage or housing behind it.
    ///
    /// This is the *initial* priority a newly placed instance takes; the player
    /// can change it per building afterwards. Defaults to
    /// [`DEFAULT_BUILDING_PRIORITY`] so existing packs load unchanged and every
    /// unauthored building competes on equal footing.
    #[serde(default = "default_priority")]
    pub default_priority: u8,
    /// Build slots this project adds to the colony on completion (issue #306).
    ///
    /// Non-zero makes this a **site-preparation project rather than a
    /// building**: it completes into [`crate::colony::Colony::slot_capacity`]
    /// and never becomes a [`PlacedBuilding`], so it occupies no slot, employs
    /// nobody, and runs no recipe. This is how a colony grows past
    /// [`BASE_SLOT_CAPACITY`] — capacity is bought with construction materials,
    /// not handed out by difficulty preset.
    ///
    /// Such a project must be authored with `slot_cost: 0`. A slot-granting
    /// project that consumed a slot would deadlock a full colony: the only way
    /// to make room would need room it does not have.
    ///
    /// `0` — the default — leaves a building behaving exactly as before.
    ///
    /// [`PlacedBuilding`]: crate::colony::PlacedBuilding
    /// [`BASE_SLOT_CAPACITY`]: crate::colony::BASE_SLOT_CAPACITY
    #[serde(default)]
    pub grants_slot_capacity: u32,
    /// Whether this building is part of the **landing kit** every colony starts
    /// with (issue #317).
    ///
    /// The kit is meant to cover every *basic* resource — housing, life support,
    /// power, water, food, building materials, research — so a new colony can
    /// always run its own bootstrap loop rather than discovering on sol 1 that it
    /// cannot make oxygen. Which buildings those are is a curation decision, so it
    /// lives here in content rather than being inferred from categories.
    ///
    /// `false` — the default — leaves a building to be built normally.
    #[serde(default)]
    pub starter_kit: bool,
    /// Per-resource banking capacity this building grants (issue #348).
    ///
    /// A colony-local resource (`power`, `water`, ...) is still a per-sol flow
    /// by default — unbanked surplus evaporates at the end of the sol, exactly
    /// as before this field existed. Building something with a `storage` entry
    /// for a resource id is what converts that flow into a stock: up to the
    /// summed capacity across every such building the colony has, the amount
    /// on hand survives into the next sol instead of being cleared.
    ///
    /// Storage is deliberately modelled as a plain building attribute — the
    /// same `Vec<Ingredient>` shape as [`Self::maintenance`] — rather than a
    /// third [`ResourceKind`] variant: whether a resource carries over is a
    /// property of what the *player has built*, not of how the resource is
    /// authored. `Ingredient::quantity` here means banking capacity, not a
    /// per-sol amount.
    ///
    /// Capacity counts every completed instance of the building regardless of
    /// staffing or `paused` state — a battery holds its charge whether or not
    /// its (nonexistent) crew is on shift.
    ///
    /// Empty (the default) means this building grants no storage.
    #[serde(default)]
    pub storage: Vec<Ingredient>,
    /// Contamination this project removes from its colony's hex on
    /// completion, in the same `[0.0, 1.0]` severity units as
    /// [`crate::map::HexCell::contamination`] (issue #388).
    ///
    /// Non-zero makes this a **remediation project rather than a building**,
    /// the same shape [`Self::grants_slot_capacity`] gives site preparation
    /// (issue #306): it completes into a reduction of the colony's hex's
    /// contamination and never becomes a [`PlacedBuilding`], so it occupies
    /// no slot, employs nobody, and runs no recipe.
    ///
    /// `0.0` — the default — leaves a building behaving exactly as before.
    #[serde(default)]
    pub contamination_reduction: f32,
    /// Maximum number of instances a single site (colony or outpost) may have.
    ///
    /// `None` — the default — means unlimited, which is how every building
    /// behaved before this field existed.
    ///
    /// The cap counts **completed instances plus anything already queued**, so
    /// a player cannot sidestep it by enqueueing several copies in one turn and
    /// letting them all complete. It is enforced per site rather than per
    /// game: two colonies may each have their own `colony_hq`, which is the
    /// point of a headquarters.
    ///
    /// Which buildings are capped is a design decision about the content, not
    /// about the engine, so it is authored here rather than special-cased by id
    /// in the command handlers.
    ///
    /// Note this is *not* enforced by the balance harness, which assembles a
    /// synthetic colony directly from a check bundle rather than going through
    /// [`crate::Command`] — a bundle may still model several instances to
    /// approximate some flow, and that stays a modelling choice.
    #[serde(default)]
    pub max_instances: Option<u32>,
    /// Conditions the site must satisfy before this can be built (issue #410).
    ///
    /// Empty — the default — means buildable anywhere, which is how every
    /// building behaved before this field existed. All listed requirements
    /// must hold; there is no "any of" across entries, only within a single
    /// [`SiteRequirement::Terrain`]'s `any_of`.
    #[serde(default)]
    pub site_requirements: Vec<SiteRequirement>,
    /// How this building's output responds to where it stands (issue #411).
    ///
    /// `None` — the default — means output is independent of site, which is
    /// how every building behaved before this field existed.
    ///
    /// Applies to **both** the building's grid-capacity contribution
    /// (`power_delta`) and its recipes' outputs. Scaling only one would make a
    /// generator that either supplies headroom it cannot fill or fills
    /// headroom it never supplied.
    #[serde(default)]
    pub output_scaling: Option<SiteScaling>,
}

/// Highest (numerically largest, lowest-urgency) staffing priority (issue #307).
///
/// Nine bands is a starting point rather than a hard design constraint — widen
/// it if play shows the granularity is too coarse.
pub const MAX_BUILDING_PRIORITY: u8 = 9;

/// Staffing priority a building takes when its content pack doesn't say
/// (issue #307) — the middle of the range, so unauthored buildings neither
/// starve nor pre-empt anything.
pub const DEFAULT_BUILDING_PRIORITY: u8 = 5;

fn default_priority() -> u8 {
    DEFAULT_BUILDING_PRIORITY
}

fn default_labor() -> u32 {
    1
}

fn default_slot_cost() -> u32 {
    1
}

fn default_construction_turns() -> u32 {
    1
}

/// Blueprint for an orbital station loadable from a content YAML pack.
///
/// Defines the resource costs and build time for constructing one station type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbitalStationBlueprint {
    /// Unique identifier (referenced by `Command::BeginOrbitalConstruction`).
    pub id: String,
    /// Human-readable name shown in the build menu.
    pub name: String,
    /// Station specialisation role produced by this blueprint.
    pub station_type: crate::orbital::StationType,
    /// Default orbit band suggested for this blueprint.
    pub default_orbit: crate::orbital::OrbitType,
    /// Commodity costs deducted from the colony pool when construction begins.
    ///
    /// Each entry is `(commodity_id, quantity)`.
    #[serde(default)]
    pub commodity_costs: Vec<(String, f32)>,
    /// Strategic months required to complete construction.
    #[serde(default = "default_build_months")]
    pub build_months: u32,
}

fn default_build_months() -> u32 {
    3
}

/// Action template for a default directive — a subset of [`crate::Command`]
/// variants that make sense as colony-automation actions and can be represented
/// without a specific colony ID (which is filled in at founding time).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum DefaultAction {
    /// Run the colony-sol advance as the automated action.
    AdvanceColonySol,
    /// Assign labour to a named slot (fraction of available labour).
    AssignLabourFraction {
        /// Production slot name (e.g. `"life_support"`).
        slot: String,
        /// Fraction of available labour to allocate, in `[0.0, 1.0]`.
        fraction: f32,
    },
}

/// Template for a default directive inserted into every newly-founded colony.
///
/// The `predicate` and `action` are instantiated with the new colony's ID when
/// `Command::FoundColony` is processed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DefaultDirectiveDef {
    /// Human-readable label / intent description for this directive.
    pub label: String,
    /// Condition that must hold for the action to fire each sol.
    pub predicate: crate::predicate::Predicate,
    /// Action template to execute when the predicate matches.
    pub action: DefaultAction,
    /// Evaluation priority — higher = checked first.
    #[serde(default)]
    pub priority: u8,
}

/// An authored celestial body that seeds `SystemState.node_map` at
/// bootstrap. Field names deliberately mirror `outpost_core::system::Body`
/// so the loader can map records 1:1 without a translation table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemBodyDef {
    /// Display name.
    pub name: String,
    /// Body kind (mirrors `outpost_core::system::BodyKind` `snake_case` tags).
    pub kind: crate::system::BodyKind,
    /// Assigned system-role. Defaults to `Unassigned` when omitted.
    #[serde(default = "SystemBodyDef::default_role")]
    pub role: crate::system::SystemRole,
    /// Distance from the primary in AU.
    pub distance_au: f32,
    /// Atmospheric thickness/density band (issue #197).
    #[serde(default = "SystemBodyDef::default_atmosphere_density")]
    pub atmosphere_density: crate::system::AtmosphereDensity,
    /// Atmospheric chemical hazard band (issue #197).
    #[serde(default = "SystemBodyDef::default_atmosphere_hazard")]
    pub atmosphere_hazard: crate::system::AtmosphereHazard,
    /// Surface temperature band.
    #[serde(default = "SystemBodyDef::default_temperature")]
    pub temperature: crate::system::TemperatureBand,
    /// Surface gravity as a fraction of Earth-g.
    #[serde(default = "SystemBodyDef::default_gravity_g")]
    pub gravity_g: f32,
    /// Ambient radiation exposure level.
    #[serde(default = "SystemBodyDef::default_radiation")]
    pub radiation: crate::system::RadiationLevel,
    /// Surface/composition archetype (issue #196). Defaults to
    /// `Unclassified`, which never biases downstream systems.
    #[serde(default = "SystemBodyDef::default_subtype")]
    pub subtype: crate::system::PlanetarySubtype,
    /// Whether rotation is tidally locked to the orbit. Flavor-only.
    #[serde(default)]
    pub tidally_locked: bool,
    /// Axial tilt in degrees. Flavor-only.
    #[serde(default = "SystemBodyDef::default_axial_tilt_deg")]
    pub axial_tilt_deg: f32,
    /// Rotation period in hours. Flavor-only.
    #[serde(default = "SystemBodyDef::default_rotation_period_hours")]
    pub rotation_period_hours: f32,
    /// Number of natural satellites orbiting this body. Flavor stat.
    #[serde(default)]
    pub moon_count: u32,
    /// Display `name` of the body this one orbits, if any, resolved to a
    /// live `BodyId` after every body in the system has been seeded (see
    /// `outpost_tauri::commands::seed_system_from_content`). Lets a `Moon`
    /// authored earlier in the file name a parent authored later.
    #[serde(default)]
    pub parent_body: Option<String>,
    /// Per-category production modifiers (issue #184) — mirrors
    /// `outpost_core::system::Body::modifiers`.
    #[serde(default)]
    pub modifiers: Vec<crate::system::BodyModifier>,
}

impl SystemBodyDef {
    fn default_role() -> crate::system::SystemRole {
        crate::system::SystemRole::Unassigned
    }
    fn default_atmosphere_density() -> crate::system::AtmosphereDensity {
        crate::system::AtmosphereDensity::Vacuum
    }
    fn default_atmosphere_hazard() -> crate::system::AtmosphereHazard {
        crate::system::AtmosphereHazard::None
    }
    fn default_temperature() -> crate::system::TemperatureBand {
        crate::system::TemperatureBand::Temperate
    }
    fn default_gravity_g() -> f32 {
        1.0
    }
    fn default_radiation() -> crate::system::RadiationLevel {
        crate::system::RadiationLevel::Low
    }
    fn default_subtype() -> crate::system::PlanetarySubtype {
        crate::system::PlanetarySubtype::Unclassified
    }
    fn default_axial_tilt_deg() -> f32 {
        23.5
    }
    fn default_rotation_period_hours() -> f32 {
        24.0
    }
}

/// An authored star system scenario. Bootstrap picks one and populates the
/// `system_state.node_map` by iterating `bodies`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarSystemDef {
    /// Unique identifier referenced by future selection UI / seeding logic.
    pub id: String,
    /// Human-readable name (e.g. "Kepler-186", "Trappist-1").
    pub name: String,
    /// Optional flavour text shown when selecting the scenario.
    #[serde(default)]
    pub description: String,
    /// Ordered list of bodies to place in the system.
    #[serde(default)]
    pub bodies: Vec<SystemBodyDef>,
}

/// A named starter-supply package selectable during colony founding.
///
/// Quantities in `commodities` are treated as **per-100-colonist** amounts.
/// They are scaled linearly by `starting_population / 100.0` when applied
/// so packages remain balanced regardless of colony size.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyPackage {
    /// Unique identifier referenced by `Command::FoundColony*`.
    pub id: String,
    /// Human-readable name (e.g. "Lean", "Standard", "Stockpile").
    pub name: String,
    /// Short flavour / summary text shown in the founding UI.
    #[serde(default)]
    pub description: String,
    /// Commodities deposited into the new colony's pool, scaled by
    /// `starting_population / 100.0`.
    #[serde(default)]
    pub commodities: Vec<Ingredient>,
}

/// What it costs to plant a new settlement (issue #359).
///
/// Two profiles are authored: `outpost` (cheap, no population) and `colony`
/// (expensive). Promotion from outpost to colony charges the *difference*, so
/// both routes to a settled world total the same price — direct founding pays
/// it up front, an outpost spreads it and produces in between.
///
/// Amounts are absolute, not per-colonist: a transport hull is a transport
/// hull regardless of how many people ride it. Colonists are charged
/// separately, out of the sponsoring colony's population.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ColonizationCost {
    /// Profile id. The engine looks up `outpost` and `colony` by name.
    pub id: String,
    /// Human-readable name for the founding UI.
    pub name: String,
    /// Short flavour / summary text.
    #[serde(default)]
    pub description: String,
    /// Commodities consumed from the sponsoring colony's pool.
    #[serde(default)]
    pub commodities: Vec<Ingredient>,
}
