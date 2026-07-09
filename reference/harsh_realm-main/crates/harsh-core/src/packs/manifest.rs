//! Schema for pack manifests.
//!
//! Ported from `src/harsh_realm/packs/manifest.py`. Pydantic `pattern=`
//! constraints become explicit validation in [`PackManifest::validate`]
//! (invoked by [`PackManifest::from_document`], the `model_validate` analog).

use serde::{Deserialize, Serialize};

use crate::runtime::{JsonObject, JsonValue};

use super::exceptions::PackError;

/// Return whether `value` matches `^[a-z][a-z0-9-]*$`.
fn is_pack_id(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Return whether `value` matches `^\d+\.\d+\.\d+$`.
fn is_semver(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

fn default_dependency_version() -> String {
    ">=0.0.0".to_string()
}

/// A declared dependency on another pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackDependency {
    /// Lowercase kebab-case pack id required by this dependency.
    pub id: String,
    /// Version constraint, e.g. `>=1.0.0`, `==1.2.3`.
    #[serde(default = "default_dependency_version")]
    pub version: String,
}

/// Pack manifest, parsed from `pack.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackManifest {
    /// Lowercase kebab-case unique pack id.
    pub id: String,
    /// SemVer MAJOR.MINOR.PATCH.
    pub version: String,
    /// Human-readable pack name.
    pub name: String,
    /// Brief description.
    #[serde(default)]
    pub description: String,
    /// Pack authors.
    #[serde(default)]
    pub authors: Vec<String>,
    /// Declared dependencies.
    #[serde(default)]
    pub depends: Vec<PackDependency>,
    /// Pack ids that cannot be loaded with this one.
    #[serde(default)]
    pub conflicts: Vec<String>,
    /// Capability tags this pack provides.
    #[serde(default)]
    pub provides: Vec<String>,
}

impl PackManifest {
    /// Deserialize and validate a manifest document.
    pub fn from_document(document: JsonObject) -> Result<PackManifest, PackError> {
        let manifest: PackManifest = serde_json::from_value(JsonValue::Object(document))
            .map_err(|e| PackError::Load(e.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate id/version patterns and required-field constraints.
    pub fn validate(&self) -> Result<(), PackError> {
        if !is_pack_id(&self.id) {
            return Err(PackError::Load(format!("Invalid pack id: {}", self.id)));
        }
        if !is_semver(&self.version) {
            return Err(PackError::Load(format!(
                "Invalid pack version: {}",
                self.version
            )));
        }
        if self.name.is_empty() {
            return Err(PackError::Load("Pack name must not be empty".to_string()));
        }
        for dependency in &self.depends {
            if !is_pack_id(&dependency.id) {
                return Err(PackError::Load(format!(
                    "Invalid dependency pack id: {}",
                    dependency.id
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(json: serde_json::Value) -> JsonObject {
        match json {
            serde_json::Value::Object(m) => m,
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn valid_manifest_parses_with_defaults() {
        let manifest = PackManifest::from_document(doc(serde_json::json!({
            "id": "xwn-core", "version": "1.2.3", "name": "XWN Core",
        })))
        .unwrap();
        assert_eq!(manifest.id, "xwn-core");
        assert!(manifest.depends.is_empty());
        assert_eq!(manifest.description, "");
    }

    #[test]
    fn dependency_version_defaults() {
        let dep: PackDependency =
            serde_json::from_value(serde_json::json!({"id": "base"})).unwrap();
        assert_eq!(dep.version, ">=0.0.0");
    }

    #[test]
    fn invalid_id_and_version_rejected() {
        assert!(PackManifest::from_document(doc(serde_json::json!({
            "id": "Bad_Id", "version": "1.0.0", "name": "x",
        })))
        .is_err());
        assert!(PackManifest::from_document(doc(serde_json::json!({
            "id": "ok", "version": "1.0", "name": "x",
        })))
        .is_err());
        assert!(PackManifest::from_document(doc(serde_json::json!({
            "id": "ok", "version": "1.0.0", "name": "",
        })))
        .is_err());
    }
}
