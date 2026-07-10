//! Pure content-pack loader: accepts raw YAML bytes, returns validated registries.
//!
//! No I/O is performed here — callers inject file contents so `outpost_core`
//! remains free of `std::fs` dependencies.  Disk reading is the harness's job.

use std::collections::HashMap;

use super::{
    error::ContentError,
    registry::ContentRegistry,
    types::{BuildingDef, CommodityDef, PackManifest, RecipeDef},
};

/// Named raw file: a `(filename, yaml_text)` pair.
pub type RawFile<'a> = (&'a str, &'a str);

/// Loads and validates a single content pack from a set of raw YAML files.
///
/// Files should include `pack.yaml` (manifest) plus any content files
/// (`commodities.yaml`, `recipes.yaml`, `buildings.yaml`, …).
///
/// Later packs merged with [`ContentRegistry::merge`] win on ID collision.
pub struct PackLoader;

impl PackLoader {
    /// Parse and validate a pack from raw file bytes.
    ///
    /// Returns a [`ContentRegistry`] populated with all valid records, or a
    /// [`ContentError`] describing the first validation failure encountered.
    ///
    /// # Errors
    ///
    /// Returns `ContentError` on parse failure, missing required fields,
    /// duplicate IDs within the pack, or unknown enum values.
    pub fn load(files: &[RawFile<'_>]) -> Result<ContentRegistry, ContentError> {
        // ── 1. Locate and parse the manifest ──────────────────────────────
        let manifest = files
            .iter()
            .find(|(name, _)| *name == "pack.yaml")
            .map(|(name, text)| parse_yaml::<PackManifest>(name, text))
            .transpose()?
            .ok_or(ContentError::MissingManifest)?;

        // ── 2. Parse typed tables ─────────────────────────────────────────
        let commodities =
            collect_table::<CommodityDef>(files, &["commodities.yaml", "resources.yaml"])?;
        let recipes = collect_table::<RecipeDef>(files, &["recipes.yaml"])?;
        let buildings = collect_table::<BuildingDef>(files, &["buildings.yaml"])?;

        // ── 3. Cross-reference validation ─────────────────────────────────
        let commodity_ids: std::collections::HashSet<&str> =
            commodities.values().map(|c| c.id.as_str()).collect();

        for recipe in recipes.values() {
            let all_refs = recipe.inputs.iter().chain(recipe.outputs.iter());
            for ing in all_refs {
                if !commodity_ids.contains(ing.id.as_str()) {
                    return Err(ContentError::UnknownCommodityRef {
                        file: "recipes.yaml".to_string(),
                        id: recipe.id.clone(),
                        commodity_id: ing.id.clone(),
                    });
                }
            }
        }

        Ok(ContentRegistry {
            manifest,
            commodities,
            recipes,
            buildings,
            orbital_blueprints: std::collections::HashMap::new(),
        })
    }
}

// ── helpers ────────────────────────────────────────────────────────────────

/// Deserialise a single YAML file into `T`.
fn parse_yaml<T: serde::de::DeserializeOwned>(file: &str, text: &str) -> Result<T, ContentError> {
    serde_yaml::from_str(text).map_err(|e| ContentError::ParseError {
        file: file.to_string(),
        message: e.to_string(),
    })
}

/// Deserialise a list-typed YAML file into `Vec<T>`.
fn parse_list<T: serde::de::DeserializeOwned>(
    file: &str,
    text: &str,
) -> Result<Vec<T>, ContentError> {
    serde_yaml::from_str(text).map_err(|e| ContentError::ParseError {
        file: file.to_string(),
        message: e.to_string(),
    })
}

/// Collect records from all matching file names, checking for intra-pack duplicates.
fn collect_table<T>(
    files: &[RawFile<'_>],
    names: &[&str],
) -> Result<HashMap<String, T>, ContentError>
where
    T: serde::de::DeserializeOwned + HasId,
{
    let mut map: HashMap<String, (String, T)> = HashMap::new(); // id → (file, record)

    for (file_name, text) in files {
        if !names.contains(file_name) {
            continue;
        }
        let records: Vec<T> = parse_list(file_name, text)?;
        for record in records {
            let id = record.id().to_string();
            if let Some((prior_file, _)) = map.get(&id) {
                return Err(ContentError::DuplicateId {
                    file: (*file_name).to_string(),
                    id,
                    prior_file: prior_file.clone(),
                });
            }
            map.insert(id, ((*file_name).to_string(), record));
        }
    }

    Ok(map.into_iter().map(|(id, (_, rec))| (id, rec)).collect())
}

/// Trait implemented by every content record that carries a string `id` field.
pub(super) trait HasId {
    /// Return the record's unique identifier.
    fn id(&self) -> &str;
}

impl HasId for CommodityDef {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for RecipeDef {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for BuildingDef {
    fn id(&self) -> &str {
        &self.id
    }
}
