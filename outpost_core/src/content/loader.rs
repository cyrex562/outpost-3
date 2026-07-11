//! Pure content-pack loader: accepts raw YAML bytes, returns validated registries.
//!
//! No I/O is performed here — callers inject file contents so `outpost_core`
//! remains free of `std::fs` dependencies.  Disk reading is the harness's job.

use std::collections::HashMap;

use super::{
    error::ContentError,
    registry::ContentRegistry,
    types::{
        BuildingDef, CommodityDef, DefaultDirectiveDef, PackManifest, RecipeDef, StarSystemDef,
        SupplyPackage,
    },
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
        let default_directives =
            collect_list::<DefaultDirectiveDef>(files, &["default_directives.yaml"])?;
        let supply_packages = collect_table::<SupplyPackage>(files, &["supplies.yaml"])?;
        let star_systems = collect_table::<StarSystemDef>(files, &["systems.yaml"])?;

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

        for pkg in supply_packages.values() {
            for ing in &pkg.commodities {
                if !commodity_ids.contains(ing.id.as_str()) {
                    return Err(ContentError::UnknownCommodityRef {
                        file: "supplies.yaml".to_string(),
                        id: pkg.id.clone(),
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
            default_directives,
            supply_packages,
            star_systems,
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

impl HasId for SupplyPackage {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for StarSystemDef {
    fn id(&self) -> &str {
        &self.id
    }
}

/// Collect records from all matching file names into a plain `Vec`.
///
/// Unlike [`collect_table`], no deduplication is performed — order is preserved.
fn collect_list<T>(files: &[RawFile<'_>], names: &[&str]) -> Result<Vec<T>, ContentError>
where
    T: serde::de::DeserializeOwned,
{
    let mut out = Vec::new();
    for (file_name, text) in files {
        if names.contains(file_name) {
            let records: Vec<T> = parse_list(file_name, text)?;
            out.extend(records);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::{BodyKind, SystemRole};

    fn minimal_pack_files(extra: &[(&str, &str)]) -> Vec<(String, String)> {
        let pack = "id: t\nname: T\nversion: '0.1.0'\n".to_string();
        let commodities = "\
- id: iron_ore
  name: Iron Ore
  category: metallic_ore
  base_value: 1.0
"
        .to_string();
        let mut files = vec![
            ("pack.yaml".to_string(), pack),
            ("commodities.yaml".to_string(), commodities),
        ];
        for (name, text) in extra {
            files.push(((*name).to_string(), (*text).to_string()));
        }
        files
    }

    #[test]
    fn systems_yaml_populates_registry() {
        let systems = "\
- id: kepler-186
  name: Kepler-186
  description: Well-charted G-type system.
  bodies:
    - name: Kepler-A
      kind: inner_planet
      role: raw_extraction
      distance_au: 0.4
    - name: Aurelian
      kind: gas_giant
      distance_au: 4.2
- id: trappist-1
  name: Trappist-1
  bodies:
    - name: T-1a
      kind: inner_planet
      distance_au: 0.11
";
        let owned = minimal_pack_files(&[("systems.yaml", systems)]);
        let raw: Vec<(&str, &str)> = owned
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        let registry = PackLoader::load(&raw).expect("systems must parse");
        assert_eq!(registry.star_systems().count(), 2);

        let kepler = registry.star_system("kepler-186").expect("kepler present");
        assert_eq!(kepler.name, "Kepler-186");
        assert_eq!(kepler.bodies.len(), 2);
        assert_eq!(kepler.bodies[0].name, "Kepler-A");
        assert_eq!(kepler.bodies[0].kind, BodyKind::InnerPlanet);
        assert_eq!(kepler.bodies[0].role, SystemRole::RawExtraction);
        // Second body omits role — defaults to Unassigned.
        assert_eq!(kepler.bodies[1].kind, BodyKind::GasGiant);
        assert_eq!(kepler.bodies[1].role, SystemRole::Unassigned);
    }

    #[test]
    fn systems_yaml_absence_is_not_an_error() {
        let owned = minimal_pack_files(&[]);
        let raw: Vec<(&str, &str)> = owned
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        let registry = PackLoader::load(&raw).expect("pack must parse without systems.yaml");
        assert_eq!(registry.star_systems().count(), 0);
    }
}
