//! Oracle-editor API models. Ported from `models/editor_api/oracle.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::{JsonObject, JsonValue};

fn d_story() -> String {
    "story".to_string()
}
fn d_active() -> String {
    "active".to_string()
}

/// Thread creation payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleThreadCreateRequest {
    /// Title.
    pub title: String,
    /// Type (default `"story"`).
    #[serde(default = "d_story")]
    pub r#type: String,
    /// Status (default `"active"`).
    #[serde(default = "d_active")]
    pub status: String,
    /// Progress.
    #[serde(default)]
    pub progress: i32,
}

/// Thread update payload.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OracleThreadUpdateRequest {
    /// Title.
    #[serde(default)]
    pub title: Option<String>,
    /// Type.
    #[serde(default)]
    pub r#type: Option<String>,
    /// Status.
    #[serde(default)]
    pub status: Option<String>,
    /// Progress.
    #[serde(default)]
    pub progress: Option<i32>,
}

impl OracleThreadUpdateRequest {
    /// Return only the fields explicitly provided by the caller.
    pub fn to_updates(&self) -> JsonObject {
        let mut u = JsonObject::new();
        if let Some(v) = &self.title {
            u.insert("title".into(), JsonValue::from(v.clone()));
        }
        if let Some(v) = &self.r#type {
            u.insert("type".into(), JsonValue::from(v.clone()));
        }
        if let Some(v) = &self.status {
            u.insert("status".into(), JsonValue::from(v.clone()));
        }
        if let Some(v) = self.progress {
            u.insert("progress".into(), JsonValue::from(v));
        }
        u
    }
}

/// Oracle NPC creation payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleNpcCreateRequest {
    /// Name.
    pub name: String,
    /// Notes.
    #[serde(default)]
    pub notes: String,
    /// Linked entity id.
    #[serde(default)]
    pub entity_id: Option<String>,
}

/// Oracle NPC update payload.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OracleNpcUpdateRequest {
    /// Name.
    #[serde(default)]
    pub name: Option<String>,
    /// Status.
    #[serde(default)]
    pub status: Option<String>,
    /// Notes.
    #[serde(default)]
    pub notes: Option<String>,
    /// Linked entity id.
    #[serde(default)]
    pub entity_id: Option<String>,
}

impl OracleNpcUpdateRequest {
    /// Return only the fields explicitly provided by the caller.
    pub fn to_updates(&self) -> JsonObject {
        let mut u = JsonObject::new();
        for (k, v) in [
            ("name", &self.name),
            ("status", &self.status),
            ("notes", &self.notes),
            ("entity_id", &self.entity_id),
        ] {
            if let Some(val) = v {
                u.insert(k.into(), JsonValue::from(val.clone()));
            }
        }
        u
    }
}

/// Oracle state update payload.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OracleStateUpdateRequest {
    /// New chaos factor.
    #[serde(default)]
    pub chaos_factor: Option<i32>,
}
