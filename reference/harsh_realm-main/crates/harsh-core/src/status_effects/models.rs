//! Runtime models for active status effects.
//!
//! Ported from `src/harsh_realm/status_effects/models.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// An applied status effect on an entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveStatusEffect {
    /// Row id of the active effect.
    pub id: i64,
    /// Owning entity id.
    pub entity_id: String,
    /// Content id of the applied effect.
    pub effect_id: String,
    /// World tick at which the effect was applied.
    pub applied_at_tick: i32,
    /// World tick at which the effect expires, if bounded.
    pub expires_at_tick: Option<i32>,
    /// Optional source descriptor.
    pub source: Option<String>,
    /// Arbitrary per-instance data (procedure values).
    #[serde(default)]
    pub data: JsonObject,
}
