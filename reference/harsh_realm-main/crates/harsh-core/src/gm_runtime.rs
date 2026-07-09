//! GM scene-local runtime state models.
//!
//! Ported from `src/harsh_realm/models/gm_runtime.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// Pending hostile encounter staged before combat starts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingEncounterState {
    /// Serialised creature records.
    pub creatures: Vec<JsonObject>,
    /// `"player_surprise"` | `"enemy_surprise"` | `"mutual_awareness"`.
    pub awareness_result: String,
    /// Terrain identifier.
    pub terrain: String,
    /// World-cell feature ids (passed to the battle-grid position algorithm).
    #[serde(default)]
    pub features: Vec<String>,
    /// Cell column.
    #[serde(default, alias = "q")]
    pub cell_q: i32,
    /// Cell row.
    #[serde(default, alias = "r")]
    pub cell_r: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_encounter_defaults() {
        let json = r#"{"creatures":[],"awareness_result":"mutual_awareness","terrain":"plains"}"#;
        let state: PendingEncounterState = serde_json::from_str(json).unwrap();
        assert_eq!(state.cell_q, 0);
        assert_eq!(state.cell_r, 0);
        assert_eq!(state.terrain, "plains");
    }

    #[test]
    fn pending_encounter_alias() {
        let json = r#"{"creatures":[],"awareness_result":"player_surprise","terrain":"ruins","q":3,"r":4}"#;
        let state: PendingEncounterState = serde_json::from_str(json).unwrap();
        assert_eq!(state.cell_q, 3);
        assert_eq!(state.cell_r, 4);
    }
}
