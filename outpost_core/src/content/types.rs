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
}

fn default_cycle_sols() -> u32 {
    1
}

/// Category of a building (production, storage, housing, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingCategory {
    /// Produces commodities from inputs.
    Production,
    /// Stores commodities.
    Storage,
    /// Houses population.
    Housing,
    /// Produces power.
    Power,
    /// Research facility.
    Research,
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
