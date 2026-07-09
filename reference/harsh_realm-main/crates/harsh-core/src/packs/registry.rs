//! Pack registry with dependency validation and load-order resolution.
//!
//! Ported from `src/harsh_realm/packs/registry.py`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

use super::exceptions::PackError;
use super::loader::{load_pack, Pack};
use super::version::satisfies;

/// Content id conflict reported across loaded packs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdConflict {
    /// The qualified id appearing in multiple packs.
    pub qualified_id: String,
    /// Pack ids that define the conflicting record.
    pub pack_ids: Vec<String>,
}

/// Validated collection of packs in dependency-safe load order.
pub struct PackRegistry {
    packs: Vec<Pack>,
    index_by_id: BTreeMap<String, usize>,
    load_order: Vec<usize>,
}

impl PackRegistry {
    /// Create a registry from already loaded packs.
    ///
    /// Packs are given in user-requested order; independent packs preserve that
    /// order while dependencies are moved before dependents.
    pub fn new(packs: Vec<Pack>) -> Result<PackRegistry, PackError> {
        let index_by_id = index_packs(&packs)?;
        let mut registry = PackRegistry {
            packs,
            index_by_id,
            load_order: Vec::new(),
        };
        registry.validate_dependencies()?;
        registry.validate_conflicts()?;
        registry.load_order = registry.resolve_load_order()?;
        Ok(registry)
    }

    /// Load selected packs from `packs_root` and validate the registry.
    pub fn from_directory(packs_root: &Path, pack_ids: &[String]) -> Result<PackRegistry, PackError> {
        let mut packs = Vec::new();
        for pack_id in pack_ids {
            packs.push(load_pack(&packs_root.join(pack_id))?);
        }
        PackRegistry::new(packs)
    }

    /// Return packs with dependencies before dependents.
    pub fn packs_in_load_order(&self) -> Vec<&Pack> {
        self.load_order.iter().map(|&i| &self.packs[i]).collect()
    }

    /// Return a loaded pack by id, or `None` if absent.
    pub fn get_pack(&self, pack_id: &str) -> Option<&Pack> {
        self.index_by_id.get(pack_id).map(|&i| &self.packs[i])
    }

    /// Return content records whose qualified ids appear in multiple packs.
    pub fn detect_id_conflicts(&self) -> Vec<IdConflict> {
        let mut pack_ids_by_record_id: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for &i in &self.load_order {
            let pack = &self.packs[i];
            for category in pack.list_categories() {
                for (_slug, record) in pack.list_records(&category) {
                    if let Some(qualified_id) =
                        record.get("_qualified_id").and_then(|v| v.as_str())
                    {
                        pack_ids_by_record_id
                            .entry(qualified_id.to_string())
                            .or_default()
                            .push(pack.manifest.id.clone());
                    }
                }
            }
        }
        pack_ids_by_record_id
            .into_iter()
            .filter(|(_, pack_ids)| pack_ids.len() > 1)
            .map(|(qualified_id, pack_ids)| IdConflict {
                qualified_id,
                pack_ids,
            })
            .collect()
    }

    /// Return a content record by fully qualified id.
    pub fn get_record(&self, qualified_id: &str) -> Option<JsonObject> {
        let (pack_id, category, slug) = parse_qualified_id(qualified_id)?;
        let pack = self.get_pack(&pack_id)?;
        pack.get_record(&category, &slug)
    }

    /// Return category records across packs in registry load order.
    pub fn list_records(&self, category: &str, pack_id: Option<&str>) -> Vec<JsonObject> {
        if let Some(pack_id) = pack_id {
            return match self.get_pack(pack_id) {
                Some(pack) => pack.list_records(category).into_iter().map(|(_s, r)| r).collect(),
                None => Vec::new(),
            };
        }
        let mut records = Vec::new();
        for &i in &self.load_order {
            records.extend(self.packs[i].list_records(category).into_iter().map(|(_s, r)| r));
        }
        records
    }

    /// Return qualified ids for a category across all loaded packs.
    pub fn list_qualified_ids(&self, category: &str) -> Vec<String> {
        self.list_records(category, None)
            .iter()
            .filter_map(|record| {
                record
                    .get("_qualified_id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .collect()
    }

    fn validate_dependencies(&self) -> Result<(), PackError> {
        for pack in &self.packs {
            for dependency in &pack.manifest.depends {
                let Some(&dep_index) = self.index_by_id.get(&dependency.id) else {
                    return Err(PackError::Resolution(format!(
                        "Pack {} depends on missing pack {}",
                        pack.manifest.id, dependency.id
                    )));
                };
                let dep_pack = &self.packs[dep_index];
                let satisfied = satisfies(&dep_pack.manifest.version, &dependency.version)
                    .map_err(|_| {
                        PackError::Resolution(format!(
                            "Pack {} has invalid dependency constraint for {}: {}",
                            pack.manifest.id, dependency.id, dependency.version
                        ))
                    })?;
                if !satisfied {
                    return Err(PackError::Resolution(format!(
                        "Pack {} requires {} {}, but loaded {}",
                        pack.manifest.id,
                        dependency.id,
                        dependency.version,
                        dep_pack.manifest.version
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_conflicts(&self) -> Result<(), PackError> {
        for pack in &self.packs {
            for conflict_id in &pack.manifest.conflicts {
                if self.index_by_id.contains_key(conflict_id) {
                    return Err(PackError::Resolution(format!(
                        "Pack conflict between {} and {}",
                        pack.manifest.id, conflict_id
                    )));
                }
            }
        }
        Ok(())
    }

    fn resolve_load_order(&self) -> Result<Vec<usize>, PackError> {
        let mut resolved: Vec<usize> = Vec::new();
        let mut permanent: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
        let mut temporary: Vec<usize> = Vec::new();
        for i in 0..self.packs.len() {
            self.visit_pack(i, &mut resolved, &mut permanent, &mut temporary)?;
        }
        Ok(resolved)
    }

    fn visit_pack(
        &self,
        index: usize,
        resolved: &mut Vec<usize>,
        permanent: &mut std::collections::BTreeSet<usize>,
        temporary: &mut Vec<usize>,
    ) -> Result<(), PackError> {
        if permanent.contains(&index) {
            return Ok(());
        }
        if let Some(pos) = temporary.iter().position(|&i| i == index) {
            let cycle: Vec<&str> = temporary[pos..]
                .iter()
                .chain(std::iter::once(&index))
                .map(|&i| self.packs[i].manifest.id.as_str())
                .collect();
            return Err(PackError::Resolution(format!(
                "Circular pack dependency detected: {}",
                cycle.join(" -> ")
            )));
        }

        temporary.push(index);
        for dependency in &self.packs[index].manifest.depends {
            let dep_index = self.index_by_id[&dependency.id];
            self.visit_pack(dep_index, resolved, permanent, temporary)?;
        }
        temporary.pop();

        permanent.insert(index);
        resolved.push(index);
        Ok(())
    }
}

fn index_packs(packs: &[Pack]) -> Result<BTreeMap<String, usize>, PackError> {
    let mut indexed: BTreeMap<String, usize> = BTreeMap::new();
    for (i, pack) in packs.iter().enumerate() {
        let pack_id = pack.manifest.id.clone();
        if indexed.contains_key(&pack_id) {
            return Err(PackError::Resolution(format!(
                "Duplicate pack ID loaded: {pack_id}"
            )));
        }
        indexed.insert(pack_id, i);
    }
    Ok(indexed)
}

/// Parse a qualified id into `(pack_id, category, slug)`.
pub(crate) fn parse_qualified_id(qualified_id: &str) -> Option<(String, String, String)> {
    let pack_sep = qualified_id.find(':')?;
    let record_sep = qualified_id.rfind('.')?;
    if pack_sep == 0
        || record_sep <= pack_sep + 1
        || record_sep >= qualified_id.len().saturating_sub(1)
    {
        return None;
    }
    Some((
        qualified_id[..pack_sep].to_string(),
        qualified_id[pack_sep + 1..record_sep].to_string(),
        qualified_id[record_sep + 1..].to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::manifest::{PackDependency, PackManifest};

    fn pack(id: &str, depends: Vec<&str>) -> Pack {
        let mut records = crate::packs::loader::PackRecords::new();
        let mut cat = std::collections::BTreeMap::new();
        let mut rec = JsonObject::new();
        rec.insert(
            "_qualified_id".into(),
            serde_json::json!(format!("{id}:skills.slug")),
        );
        cat.insert("slug".to_string(), rec);
        records.insert("skills".to_string(), cat);
        Pack {
            manifest: PackManifest {
                id: id.to_string(),
                version: "1.0.0".to_string(),
                name: id.to_string(),
                description: String::new(),
                authors: vec![],
                depends: depends
                    .into_iter()
                    .map(|d| PackDependency {
                        id: d.to_string(),
                        version: ">=0.0.0".to_string(),
                    })
                    .collect(),
                conflicts: vec![],
                provides: vec![],
            },
            root_path: std::path::PathBuf::from("/tmp"),
            records,
        }
    }

    #[test]
    fn dependencies_load_before_dependents() {
        // Requested order: app depends on base, but app first.
        let registry = PackRegistry::new(vec![pack("app", vec!["base"]), pack("base", vec![])]).unwrap();
        let order: Vec<&str> = registry
            .packs_in_load_order()
            .iter()
            .map(|p| p.manifest.id.as_str())
            .collect();
        assert_eq!(order, vec!["base", "app"]);
    }

    #[test]
    fn missing_dependency_errors() {
        assert!(PackRegistry::new(vec![pack("app", vec!["ghost"])]).is_err());
    }

    #[test]
    fn cycle_is_detected() {
        let result = PackRegistry::new(vec![pack("a", vec!["b"]), pack("b", vec!["a"])]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_qualified_id_rules() {
        assert_eq!(
            parse_qualified_id("xwn-core:skills.stab"),
            Some(("xwn-core".into(), "skills".into(), "stab".into()))
        );
        assert!(parse_qualified_id("nocolon.slug").is_none());
        assert!(parse_qualified_id(":skills.stab").is_none());
        assert!(parse_qualified_id("p:cat.").is_none());
    }

    #[test]
    fn get_record_resolves_qualified_id() {
        let registry = PackRegistry::new(vec![pack("xwn", vec![])]).unwrap();
        assert!(registry.get_record("xwn:skills.slug").is_some());
        assert!(registry.get_record("xwn:skills.missing").is_none());
    }
}
