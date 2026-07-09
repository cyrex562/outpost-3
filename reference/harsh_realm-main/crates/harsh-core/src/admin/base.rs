//! Row-backed admin base record. Ported from `models/admin/_base.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// Row-backed record with stable identity fields plus free-form JSON data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedJsonRecord {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Source.
    #[serde(default)]
    pub source: String,
    /// Unknown extra keys (Pydantic `extra="allow"`).
    #[serde(flatten)]
    pub extra: JsonObject,
}
