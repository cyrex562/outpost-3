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
    /// Generic / uncategorised.
    Other,
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
