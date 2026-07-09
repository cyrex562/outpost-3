//! Public API request models for Harsh Realm.
//!
//! Ported from `src/harsh_realm/models/public_api.py`. Pydantic field
//! validators are ported as explicit `validate()` methods callers invoke.

use serde::{Deserialize, Serialize};

use crate::api::ui::UILayoutState;
use crate::runtime::JsonObject;

fn d_twenty() -> i32 {
    20
}

/// Request body for creating a new world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateWorldRequest {
    /// World name.
    pub name: String,
    /// Width (default 20).
    #[serde(default = "d_twenty")]
    pub width: i32,
    /// Height (default 20).
    #[serde(default = "d_twenty")]
    pub height: i32,
    /// RNG seed.
    #[serde(default)]
    pub seed: Option<i64>,
    /// Pack ids to bind.
    #[serde(default)]
    pub pack_ids: Option<Vec<String>>,
}

impl CreateWorldRequest {
    /// Validate name (non-empty after trim) and dimensions (5..=100).
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("World name must not be empty.".to_string());
        }
        for dim in [self.width, self.height] {
            if !(5..=100).contains(&dim) {
                return Err("Map dimensions must be between 5 and 100.".to_string());
            }
        }
        Ok(())
    }
}

/// Request body for loading an existing world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadWorldRequest {
    /// World file path.
    pub file: String,
}

/// Request body for saving a named snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveWorldRequest {
    /// Snapshot name.
    pub name: String,
}

impl SaveWorldRequest {
    /// Validate the snapshot name is non-empty and filename-safe
    /// (letters, digits, underscores, hyphens).
    pub fn validate(&self) -> Result<(), String> {
        let value = self.name.trim();
        if value.is_empty() {
            return Err("Snapshot name must not be empty.".to_string());
        }
        if !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(
                "Snapshot name may only contain letters, digits, underscores, and hyphens."
                    .to_string(),
            );
        }
        Ok(())
    }
}

/// Typed request body for saving UI layout state.
pub type UILayoutUpdateRequest = UILayoutState;

/// Request body for creating or replacing a content override.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentOverrideRequest {
    /// Override data.
    pub data: JsonObject,
}

/// Request body for creating or replacing a procedure override.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcedureOverrideRequest {
    /// Override data.
    pub data: JsonObject,
}

/// Request body for running a procedure from the admin API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProcedureRunRequest {
    /// Inputs.
    #[serde(default)]
    pub inputs: JsonObject,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_world_validation() {
        let ok = CreateWorldRequest {
            name: "Ashfall".into(),
            width: 20,
            height: 20,
            seed: None,
            pack_ids: None,
        };
        assert!(ok.validate().is_ok());
        let blank = CreateWorldRequest {
            name: "   ".into(),
            ..ok.clone()
        };
        assert!(blank.validate().is_err());
        let too_big = CreateWorldRequest {
            width: 200,
            ..ok.clone()
        };
        assert!(too_big.validate().is_err());
    }

    #[test]
    fn snapshot_name_validation() {
        let good = SaveWorldRequest {
            name: "save_1-final".into(),
        };
        assert!(good.validate().is_ok());
        let bad = SaveWorldRequest {
            name: "save 1!".into(),
        };
        assert!(bad.validate().is_err());
    }
}
