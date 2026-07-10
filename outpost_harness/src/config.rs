//! Colony configuration types for the balance harness.
//!
//! A colony config is a YAML file that lists placed buildings (with their
//! active recipe) and any explicit import flows.

use serde::{Deserialize, Serialize};

/// One placed building in the colony, with an optional active recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingEntry {
    /// Id of the building definition in the content pack.
    pub building_id: String,
    /// Id of the recipe this building is running, if any.
    #[serde(default)]
    pub recipe_id: Option<String>,
}

/// An explicit external import flow added to the colony's supply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEntry {
    /// Commodity id (must exist in the content pack).
    pub commodity_id: String,
    /// Rate in units per colony-sol (must be positive).
    pub rate: f64,
}

/// Top-level colony configuration parsed from a YAML config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColonyConfig {
    /// Buildings placed in the colony.
    #[serde(default)]
    pub buildings: Vec<BuildingEntry>,
    /// External import flows supplementing building production.
    #[serde(default)]
    pub imports: Vec<ImportEntry>,
}
