//! Faction editor admin models. Ported from `models/admin/faction.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// Typed faction asset row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionAssetRecord {
    /// Id.
    pub id: String,
    /// Asset type.
    pub asset_type: String,
    /// Category.
    pub category: String,
    /// HP.
    pub hp: i32,
    /// Max HP.
    pub max_hp: i32,
    /// Location column.
    #[serde(default)]
    pub location_q: Option<i32>,
    /// Location row.
    #[serde(default)]
    pub location_r: Option<i32>,
    /// Data payload.
    #[serde(default)]
    pub data: JsonObject,
}

/// Typed faction relation row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactionRelationRecord {
    /// First faction id.
    pub faction_a: String,
    /// Second faction id.
    pub faction_b: String,
    /// Disposition label.
    pub disposition: String,
    /// Relation history.
    #[serde(default)]
    pub history: Vec<String>,
}

/// Typed faction editor row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionRecord {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
    /// HP.
    pub hp: i32,
    /// Max HP.
    pub max_hp: i32,
    /// Force.
    pub force: i32,
    /// Cunning.
    pub cunning: i32,
    /// Wealth.
    pub wealth: i32,
    /// XP.
    pub xp: i32,
    /// Home column.
    #[serde(default)]
    pub home_q: Option<i32>,
    /// Home row.
    #[serde(default)]
    pub home_r: Option<i32>,
    /// Goals.
    #[serde(default)]
    pub goals: Vec<String>,
    /// Tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Data payload.
    #[serde(default)]
    pub data: JsonObject,
}

/// Faction record plus assets and relations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionDetail {
    /// Embedded faction record.
    #[serde(flatten)]
    pub faction: FactionRecord,
    /// Assets.
    #[serde(default)]
    pub assets: Vec<FactionAssetRecord>,
    /// Relations.
    #[serde(default)]
    pub relations: Vec<FactionRelationRecord>,
}

/// Faction creation response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactionCreated {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
}

/// Faction asset creation response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactionAssetCreated {
    /// Asset id.
    pub id: String,
    /// Owning faction id.
    pub faction_id: String,
}
