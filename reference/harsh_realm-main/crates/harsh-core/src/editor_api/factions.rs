//! Faction-editor API models. Ported from `models/editor_api/factions.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::{JsonObject, JsonValue};

/// Editable faction fields (all optional for partial updates).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FactionUpdateRequest {
    /// Name.
    #[serde(default)]
    pub name: Option<String>,
    /// HP.
    #[serde(default)]
    pub hp: Option<i32>,
    /// Max HP.
    #[serde(default)]
    pub max_hp: Option<i32>,
    /// Force.
    #[serde(default)]
    pub force: Option<i32>,
    /// Cunning.
    #[serde(default)]
    pub cunning: Option<i32>,
    /// Wealth.
    #[serde(default)]
    pub wealth: Option<i32>,
    /// XP.
    #[serde(default)]
    pub xp: Option<i32>,
    /// Home column.
    #[serde(default)]
    pub home_q: Option<i32>,
    /// Home row.
    #[serde(default)]
    pub home_r: Option<i32>,
    /// Goals.
    #[serde(default)]
    pub goals: Option<Vec<String>>,
    /// Tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

impl FactionUpdateRequest {
    /// Return only the fields explicitly provided by the caller.
    pub fn to_updates(&self) -> JsonObject {
        let mut u = JsonObject::new();
        if let Some(v) = &self.name {
            u.insert("name".into(), JsonValue::from(v.clone()));
        }
        for (k, v) in [
            ("hp", self.hp),
            ("max_hp", self.max_hp),
            ("force", self.force),
            ("cunning", self.cunning),
            ("wealth", self.wealth),
            ("xp", self.xp),
            ("home_q", self.home_q),
            ("home_r", self.home_r),
        ] {
            if let Some(val) = v {
                u.insert(k.into(), JsonValue::from(val));
            }
        }
        if let Some(g) = &self.goals {
            u.insert("goals".into(), JsonValue::from(g.clone()));
        }
        if let Some(t) = &self.tags {
            u.insert("tags".into(), JsonValue::from(t.clone()));
        }
        u
    }
}

/// Faction relation payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactionRelationUpdateRequest {
    /// Disposition label.
    pub disposition: String,
}

/// Editable faction asset fields (all optional for partial updates).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FactionAssetUpdateRequest {
    /// Asset type.
    #[serde(default)]
    pub asset_type: Option<String>,
    /// Category.
    #[serde(default)]
    pub category: Option<String>,
    /// HP.
    #[serde(default)]
    pub hp: Option<i32>,
    /// Max HP.
    #[serde(default)]
    pub max_hp: Option<i32>,
    /// Location column.
    #[serde(default)]
    pub location_q: Option<i32>,
    /// Location row.
    #[serde(default)]
    pub location_r: Option<i32>,
}

impl FactionAssetUpdateRequest {
    /// Return only the fields explicitly provided by the caller.
    pub fn to_updates(&self) -> JsonObject {
        let mut u = JsonObject::new();
        if let Some(v) = &self.asset_type {
            u.insert("asset_type".into(), JsonValue::from(v.clone()));
        }
        if let Some(v) = &self.category {
            u.insert("category".into(), JsonValue::from(v.clone()));
        }
        for (k, v) in [
            ("hp", self.hp),
            ("max_hp", self.max_hp),
            ("location_q", self.location_q),
            ("location_r", self.location_r),
        ] {
            if let Some(val) = v {
                u.insert(k.into(), JsonValue::from(val));
            }
        }
        u
    }
}
