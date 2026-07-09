//! Generic result, YAML, and world-file admin models. Ported from
//! `models/admin/common.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::{JsonObject, JsonValue};

/// Count result for reset-all admin endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResetCount {
    /// Number reset.
    pub reset: i32,
}

/// One YAML file listing entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YamlFileInfo {
    /// Path.
    pub path: String,
    /// Size in bytes.
    pub size: i64,
}

/// Inline YAML file contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YamlFileContent {
    /// Path.
    pub path: String,
    /// Content.
    pub content: String,
}

/// YAML save/upload result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YamlFileSaveResult {
    /// Path.
    pub path: String,
    /// Size in bytes.
    pub size: i64,
}

/// YAML delete result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YamlDeleteResult {
    /// Status string.
    pub status: String,
    /// Path.
    pub path: String,
}

/// Status summary for one table YAML file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YamlTableStatus {
    /// Path.
    pub path: String,
    /// Size in bytes.
    pub size: i64,
    /// Number of entries.
    pub entry_count: i32,
    /// Whether the file has a TODO marker.
    pub has_todo: bool,
}

/// Bulk YAML zip upload result.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct YamlTablesZipUploadResult {
    /// Written paths.
    #[serde(default)]
    pub written: Vec<String>,
    /// Errors.
    #[serde(default)]
    pub errors: Vec<String>,
}

/// Generic parsed YAML document for validation-only editor flows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct YamlDocument(pub JsonValue);

impl YamlDocument {
    /// Return the document as an object when the root is a mapping.
    pub fn as_object(&self) -> Option<&JsonObject> {
        self.0.as_object()
    }
}

/// Generic JSON object wrapper for typed request bodies.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonObjectDocument(pub JsonObject);

/// Simple delete acknowledgement payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteResult {
    /// Deleted id.
    pub deleted: String,
}

/// Simple success payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OkResult {
    /// Success flag.
    pub ok: bool,
}

/// One world database file available to the editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFileInfo {
    /// File path.
    pub file: String,
    /// World name.
    pub name: String,
    /// Last-modified timestamp.
    pub last_modified: String,
}

/// World clone endpoint result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldCloneResult {
    /// World name.
    pub name: String,
    /// File path.
    pub file: String,
}

/// Editor schema document with stable routing keys and flexible fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorSchemaDefinition {
    /// Routing pattern.
    pub pattern: String,
    /// Label.
    #[serde(default)]
    pub label: String,
    /// Widget id.
    #[serde(default)]
    pub widget: String,
    /// Field definitions.
    #[serde(default)]
    pub fields: Vec<JsonObject>,
    /// Unknown extra keys (Pydantic `extra="allow"`).
    #[serde(flatten)]
    pub extra: JsonObject,
}
