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
}
