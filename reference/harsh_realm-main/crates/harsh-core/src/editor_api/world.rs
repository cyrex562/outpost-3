//! World-editor API models. Ported from `models/editor_api/world.py`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// Query parameters for world clone operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldCloneQuery {
    /// New world name.
    pub new_name: String,
}

/// Metadata update payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldMetaUpdateRequest {
    /// New value.
    pub value: String,
}

/// JSON import payload.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableImportRequest {
    /// Rows to import.
    #[serde(default)]
    pub rows: Vec<JsonObject>,
}

/// Bulk import payload.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BulkTableImportRequest {
    /// Table id -> rows.
    #[serde(default)]
    pub tables: BTreeMap<String, Vec<JsonObject>>,
}
