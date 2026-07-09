//! Settlement and building entity models.
//!
//! Ported from `src/harsh_realm/models/settlement.py`. Settlements are entities
//! with child building entities; this holds the data stored in their `data`
//! columns plus a backwards-compatible summary.

use serde::{Deserialize, Serialize};

use crate::scene_data::SettlementBuilding;

/// Building tier for a settlement size, or `None` if unknown.
pub fn size_to_tier(size: &str) -> Option<&'static str> {
    match size {
        "hamlet" => Some("small"),
        "village" => Some("medium"),
        "town" | "city" => Some("large"),
        _ => None,
    }
}

/// Minimum required building types for a settlement size.
pub fn required_buildings(size: &str) -> &'static [&'static str] {
    match size {
        "hamlet" => &["healer", "general_store"],
        "village" => &["healer", "general_store", "blacksmith"],
        "town" | "city" => &["healer", "general_store", "blacksmith", "tavern"],
        _ => &[],
    }
}

/// Data stored in the `data` column of a building entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingData {
    /// healer, general_store, blacksmith, tavern.
    pub building_type: String,
    /// Display name.
    pub building_name: String,
    /// small, medium, large.
    pub tier: String,
    /// Parent settlement entity id.
    pub settlement_id: String,
    /// NPC who runs this building.
    #[serde(default)]
    pub operator_npc_id: Option<String>,
}

/// Data stored in the `data` column of a settlement entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementData {
    /// Name.
    pub name: String,
    /// hamlet, village, town, city.
    pub size: String,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Child building entity ids.
    #[serde(default)]
    pub building_ids: Vec<String>,
    /// Resident NPC entity ids.
    #[serde(default)]
    pub resident_npc_ids: Vec<String>,
}

/// Backwards-compatible settlement payload stored in cell JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettlementSummary {
    /// Name.
    pub name: String,
    /// Size.
    pub size: String,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Settlement entity id.
    pub settlement_id: String,
    /// Establishments.
    #[serde(default)]
    pub establishments: Vec<SettlementBuilding>,
    /// Resident NPC ids.
    #[serde(default)]
    pub resident_npc_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_and_required_buildings() {
        assert_eq!(size_to_tier("hamlet"), Some("small"));
        assert_eq!(size_to_tier("city"), Some("large"));
        assert_eq!(size_to_tier("nowhere"), None);
        assert_eq!(required_buildings("village").len(), 3);
        assert_eq!(required_buildings("town"), &["healer", "general_store", "blacksmith", "tavern"]);
        assert!(required_buildings("nowhere").is_empty());
    }
}
