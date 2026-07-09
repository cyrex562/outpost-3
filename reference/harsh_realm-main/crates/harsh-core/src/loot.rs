//! Loot generation result models.
//!
//! Ported from `src/harsh_realm/models/loot.py`.

use serde::{Deserialize, Serialize};

use crate::engine_runtime::{HarvestableRecord, LootItemData};

/// A single loot item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LootItem {
    /// Name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Item type.
    pub item_type: String,
    /// Value.
    pub value: i32,
    /// Structured payload.
    pub data: LootItemData,
}

/// Result of a harvesting attempt on a creature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarvestResult {
    /// Whether the harvest succeeded.
    pub success: bool,
    /// Material harvested.
    pub material: String,
    /// Narration text.
    pub narration: String,
}

/// The combined loot result from a combat encounter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LootResult {
    /// Item drops.
    pub items: Vec<LootItem>,
    /// Currency dropped.
    pub currency: i32,
    /// Harvestable materials made available.
    pub harvestable: Vec<HarvestableRecord>,
    /// Narration text.
    pub narration: String,
}
