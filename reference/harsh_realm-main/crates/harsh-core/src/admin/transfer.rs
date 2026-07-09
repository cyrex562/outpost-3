//! Import/export and world-meta admin models. Ported from
//! `models/admin/transfer.py`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// Import outcome for one table.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ImportResult {
    /// Number imported.
    pub imported: i32,
    /// Errors.
    #[serde(default)]
    pub errors: Vec<String>,
}

/// Single-table export response payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportTableResult {
    /// Table name.
    pub table: String,
    /// Row count.
    pub count: i32,
    /// Rows.
    pub rows: Vec<JsonObject>,
}

/// Per-table result for bulk import.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ImportAllTableResult {
    /// Number imported, if any.
    #[serde(default)]
    pub imported: Option<i32>,
    /// Errors.
    #[serde(default)]
    pub errors: Vec<String>,
    /// Single error, if any.
    #[serde(default)]
    pub error: Option<String>,
}

/// Bulk import response payload keyed by table name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportAllResult {
    /// Per-table results.
    pub tables: BTreeMap<String, ImportAllTableResult>,
}

/// Bulk export payload keyed by table name (free-form).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ExportAllResult {
    /// Tables (Pydantic `extra="allow"`).
    #[serde(flatten)]
    pub tables: JsonObject,
}

/// One world metadata key/value pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldMetaRecord {
    /// Key.
    pub key: String,
    /// Value.
    pub value: String,
}

/// World metadata mapping (free-form).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WorldMetaSnapshot {
    /// Metadata (Pydantic `extra="allow"`).
    #[serde(flatten)]
    pub meta: JsonObject,
}
