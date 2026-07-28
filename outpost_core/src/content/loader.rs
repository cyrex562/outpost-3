//! Pure content-pack loader: accepts raw YAML bytes, returns validated registries.
//!
//! No I/O is performed here — callers inject file contents so `outpost_core`
//! remains free of `std::fs` dependencies.  Disk reading is the harness's job.

use std::collections::HashMap;

use super::{
    error::ContentError,
    registry::ContentRegistry,
    types::{
        BuildingDef, ColonizationCost, CommodityDef, DefaultDirectiveDef, PackManifest, RecipeDef,
        ResourceDef, StarSystemDef, SupplyPackage,
    },
};
use crate::expedition::AnomalyDef;

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
        let commodities = collect_table::<CommodityDef>(files, &["commodities.yaml"])?;
        // Colony-local resources (issue #304) are a separate table, not more
        // commodities. `resources.yaml` used to be an alias for another
        // commodities file; it now has its own type.
        let resources = collect_table::<ResourceDef>(files, &["resources.yaml"])?;
        let recipes = collect_table::<RecipeDef>(files, &["recipes.yaml"])?;
        let buildings = collect_table::<BuildingDef>(files, &["buildings.yaml"])?;
        let default_directives =
            collect_list::<DefaultDirectiveDef>(files, &["default_directives.yaml"])?;
        let supply_packages = collect_table::<SupplyPackage>(files, &["supplies.yaml"])?;
        let star_systems = collect_table::<StarSystemDef>(files, &["systems.yaml"])?;
        let anomalies = collect_table::<AnomalyDef>(files, &["anomalies.yaml"])?;
        let colonization_costs = collect_table::<ColonizationCost>(files, &["colonization.yaml"])?;

        // ── 3. Cross-reference validation ─────────────────────────────────
        // An id must be declared exactly once, as either a commodity or a
        // colony resource. Declaring it as both would make pool dispatch
        // ambiguous, so that's a load error rather than a silent precedence
        // rule (issue #304).
        for id in commodities.keys() {
            if resources.contains_key(id) {
                return Err(ContentError::DuplicateId {
                    file: "resources.yaml".to_string(),
                    id: id.clone(),
                    prior_file: "commodities.yaml".to_string(),
                });
            }
        }

        let commodity_ids: std::collections::HashSet<&str> =
            commodities.values().map(|c| c.id.as_str()).collect();
        // Recipes may name either kind: a power plant outputs the `power`
        // resource, a smelter outputs a commodity.
        let producible_ids: std::collections::HashSet<&str> = commodity_ids
            .iter()
            .copied()
            .chain(resources.values().map(|r| r.id.as_str()))
            .collect();

        for recipe in recipes.values() {
            let all_refs = recipe.inputs.iter().chain(recipe.outputs.iter());
            for ing in all_refs {
                if !producible_ids.contains(ing.id.as_str()) {
                    return Err(ContentError::UnknownCommodityRef {
                        file: "recipes.yaml".to_string(),
                        id: recipe.id.clone(),
                        commodity_id: ing.id.clone(),
                    });
                }
            }
        }

        // Supply packages are physically shipped to a new colony, so unlike
        // recipes they may only name tradeable commodities — you can't load
        // `power` onto a lander.
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

        validate_colonization_costs(&colonization_costs, &commodity_ids)?;

        // Issue #196: each body's subtype must be valid for its kind, and
        // any authored `parent_body` must name another body in the same
        // system (self-references rejected too — a body can't orbit itself).
        for system in star_systems.values() {
            let body_names: std::collections::HashSet<&str> =
                system.bodies.iter().map(|b| b.name.as_str()).collect();
            for body in &system.bodies {
                if !body.subtype.compatible_with(&body.kind) {
                    return Err(ContentError::IncompatiblePlanetarySubtype {
                        file: "systems.yaml".to_string(),
                        system_id: system.id.clone(),
                        body_name: body.name.clone(),
                        kind: body.kind.clone(),
                        subtype: body.subtype,
                    });
                }
                if let Some(parent_name) = &body.parent_body {
                    if parent_name == &body.name || !body_names.contains(parent_name.as_str()) {
                        return Err(ContentError::UnknownParentBodyRef {
                            file: "systems.yaml".to_string(),
                            system_id: system.id.clone(),
                            body_name: body.name.clone(),
                            parent_name: parent_name.clone(),
                        });
                    }
                }
            }
        }

        Ok(ContentRegistry {
            manifest,
            commodities,
            resources,
            recipes,
            buildings,
            orbital_blueprints: std::collections::HashMap::new(),
            default_directives,
            supply_packages,
            star_systems,
            anomalies,
            colonization_costs,
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

impl HasId for ResourceDef {
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

/// Every commodity a colonization profile names must exist and be shippable.
///
/// Colonization costs are physically loaded onto a ship, so they carry the same
/// tradeable-commodity restriction as supply packages (issue #359).
fn validate_colonization_costs(
    costs: &std::collections::HashMap<String, ColonizationCost>,
    commodity_ids: &std::collections::HashSet<&str>,
) -> Result<(), ContentError> {
    for cost in costs.values() {
        for ing in &cost.commodities {
            if !commodity_ids.contains(ing.id.as_str()) {
                return Err(ContentError::UnknownCommodityRef {
                    file: "colonization.yaml".to_string(),
                    id: cost.id.clone(),
                    commodity_id: ing.id.clone(),
                });
            }
        }
    }
    Ok(())
}

impl HasId for ColonizationCost {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for StarSystemDef {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for AnomalyDef {
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
    use crate::system::{BodyKind, PlanetarySubtype, SystemRole};

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

    // ── Planetary subtype + parent-body validation (issue #196) ──────────────

    #[test]
    fn systems_yaml_parses_subtype_and_parent_body() {
        let systems = "\
- id: kepler-186
  name: Kepler-186
  bodies:
    - name: Aurelian
      kind: gas_giant
      distance_au: 4.2
      subtype: ice_giant
    - name: Aurelian-Moon
      kind: moon
      distance_au: 4.35
      subtype: icy
      parent_body: Aurelian
      tidally_locked: true
      moon_count: 0
";
        let owned = minimal_pack_files(&[("systems.yaml", systems)]);
        let raw: Vec<(&str, &str)> = owned
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        let registry = PackLoader::load(&raw).expect("systems must parse");
        let kepler = registry.star_system("kepler-186").expect("kepler present");
        assert_eq!(kepler.bodies[0].subtype, PlanetarySubtype::IceGiant);
        assert_eq!(kepler.bodies[1].subtype, PlanetarySubtype::Icy);
        assert_eq!(kepler.bodies[1].parent_body.as_deref(), Some("Aurelian"));
        assert!(kepler.bodies[1].tidally_locked);
    }

    #[test]
    fn systems_yaml_defaults_subtype_to_unclassified() {
        let systems = "\
- id: kepler-186
  name: Kepler-186
  bodies:
    - name: Kepler-A
      kind: inner_planet
      distance_au: 0.4
";
        let owned = minimal_pack_files(&[("systems.yaml", systems)]);
        let raw: Vec<(&str, &str)> = owned
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        let registry = PackLoader::load(&raw).expect("systems must parse");
        let kepler = registry.star_system("kepler-186").expect("kepler present");
        assert_eq!(kepler.bodies[0].subtype, PlanetarySubtype::Unclassified);
        assert_eq!(kepler.bodies[0].parent_body, None);
    }

    #[test]
    fn incompatible_subtype_is_rejected() {
        let systems = "\
- id: kepler-186
  name: Kepler-186
  bodies:
    - name: Kepler-A
      kind: inner_planet
      distance_au: 0.4
      subtype: ice_giant
";
        let owned = minimal_pack_files(&[("systems.yaml", systems)]);
        let raw: Vec<(&str, &str)> = owned
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        let err =
            PackLoader::load(&raw).expect_err("ice_giant on an inner_planet must be rejected");
        assert!(matches!(
            err,
            ContentError::IncompatiblePlanetarySubtype { .. }
        ));
    }

    #[test]
    fn unknown_parent_body_ref_is_rejected() {
        let systems = "\
- id: kepler-186
  name: Kepler-186
  bodies:
    - name: Aurelian-Moon
      kind: moon
      distance_au: 4.35
      parent_body: Nonexistent
";
        let owned = minimal_pack_files(&[("systems.yaml", systems)]);
        let raw: Vec<(&str, &str)> = owned
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        let err = PackLoader::load(&raw).expect_err("dangling parent_body must be rejected");
        assert!(matches!(err, ContentError::UnknownParentBodyRef { .. }));
    }

    #[test]
    fn self_referencing_parent_body_is_rejected() {
        let systems = "\
- id: kepler-186
  name: Kepler-186
  bodies:
    - name: Aurelian
      kind: gas_giant
      distance_au: 4.2
      parent_body: Aurelian
";
        let owned = minimal_pack_files(&[("systems.yaml", systems)]);
        let raw: Vec<(&str, &str)> = owned
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        let err =
            PackLoader::load(&raw).expect_err("self-referencing parent_body must be rejected");
        assert!(matches!(err, ContentError::UnknownParentBodyRef { .. }));
    }

    // ── Authored staffing priorities (issue #307 stage 5) ────────────────────

    /// Read the real `content/base/buildings.yaml`, or `None` when the
    /// repository layout doesn't include `content/` beside the crate.
    fn read_real_buildings_yaml() -> Option<String> {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
        let root = std::path::Path::new(&manifest).parent()?.to_path_buf();
        std::fs::read_to_string(root.join("content").join("base").join("buildings.yaml")).ok()
    }

    /// Every authored building carries an in-range staffing priority.
    ///
    /// `default_priority` is `#[serde(default)]`, so a missing one is silently
    /// the middle band rather than an error — which means a new building can be
    /// added without anyone deciding where it queues. This asserts the roster is
    /// deliberately banded instead: several distinct values in use, none of them
    /// out of range.
    #[test]
    fn every_authored_building_has_an_in_range_staffing_priority() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return; // content/ not present in this checkout layout; skip.
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        assert!(
            buildings.len() >= 40,
            "expected the full authored roster, got {}",
            buildings.len()
        );

        for b in &buildings {
            assert!(
                (1..=crate::content::types::MAX_BUILDING_PRIORITY).contains(&b.default_priority),
                "{} has default_priority {} — outside 1..={}",
                b.id,
                b.default_priority,
                crate::content::types::MAX_BUILDING_PRIORITY
            );
        }

        let distinct: std::collections::BTreeSet<u8> =
            buildings.iter().map(|b| b.default_priority).collect();
        assert!(
            distinct.len() >= 5,
            "the roster should be spread across bands, not all one value; got {distinct:?}"
        );
    }

    /// The authored ordering matches #307's stated intent: life support ahead of
    /// research, research ahead of storage and housing.
    ///
    /// Pins the *relationships* rather than the exact numbers, so the bands can be
    /// retuned freely but a change that puts a lab ahead of the oxygen scrubbers
    /// fails here instead of quietly changing who starves in a shortage.
    #[test]
    fn authored_priorities_put_life_support_ahead_of_research() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        let priority = |id: &str| {
            buildings
                .iter()
                .find(|b| b.id == id)
                .unwrap_or_else(|| panic!("{id} must exist in the roster"))
                .default_priority
        };

        // Suffocation beats every other concern.
        for life_support in ["life_support_module", "air_miner", "colony_hq"] {
            assert!(
                priority(life_support) < priority("greenhouse_dome"),
                "{life_support} must be staffed before food"
            );
        }
        // Starvation beats industry.
        assert!(priority("greenhouse_dome") < priority("smelter"));
        // Industry beats research — research is what you sacrifice in a crisis.
        assert!(priority("smelter") < priority("research_lab"));
        // And research still beats the crewless structures — #307's "a hab queues
        // behind a lab", which holds precisely because these need no crew.
        for passive in [
            "warehouse",
            "basic_habitat",
            "habitat_pod",
            "solar_array_mk1",
        ] {
            assert!(
                priority("research_lab") < priority(passive),
                "{passive} must queue behind research"
            );
        }

        // But *staffed* housing is not a passive hab. A habitat_dome yields its 60
        // housing only while crewed, so leaving it unstaffed evicts the population
        // — it must not sit in the crewless band with the pods.
        for staffed_housing in ["apartment_block", "habitat_dome"] {
            assert!(
                priority(staffed_housing) < priority("research_lab"),
                "{staffed_housing} needs crew to provide housing at all, so it must \
                 be staffed ahead of research"
            );
            assert!(
                priority(staffed_housing) < priority("basic_habitat"),
                "{staffed_housing} must outrank the crewless habs"
            );
        }
    }

    // ── Authored landing kit (issue #317) ────────────────────────────────────

    /// Read the real `content/base/recipes.yaml`, or `None` when the repository
    /// layout doesn't include `content/` beside the crate.
    fn read_real_recipes_yaml() -> Option<String> {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
        let root = std::path::Path::new(&manifest).parent()?.to_path_buf();
        std::fs::read_to_string(root.join("content").join("base").join("recipes.yaml")).ok()
    }

    /// The authored landing kit fits inside a new colony's slot budget with room
    /// left over.
    ///
    /// A kit that exactly filled the base capacity would make site preparation a
    /// mandatory first build before the player could ever place anything of their
    /// own — the opposite of what the kit is for.
    #[test]
    fn the_authored_landing_kit_fits_base_slot_capacity_with_room_spare() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return; // content/ not present in this checkout layout; skip.
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        let kit: Vec<_> = buildings.iter().filter(|b| b.starter_kit).collect();
        assert!(
            !kit.is_empty(),
            "the roster must flag a landing kit, or founding places nothing"
        );

        let total: u32 = kit.iter().map(|b| b.slot_cost).sum();
        assert!(
            total < crate::colony::BASE_SLOT_CAPACITY,
            "landing kit costs {total} slots but base capacity is {} — the player \
             must have slots left for their own first build",
            crate::colony::BASE_SLOT_CAPACITY
        );
    }

    /// The landing kit can actually produce every basic resource.
    ///
    /// This is the substance of #317's second half: "one of every building
    /// necessary to produce all basic resources". Flagging eight plausible-looking
    /// buildings isn't enough — their recipes have to close the loop, so this
    /// checks the outputs rather than the roster.
    #[test]
    fn the_authored_landing_kit_covers_every_basic_resource() {
        let (Some(byaml), Some(ryaml)) = (read_real_buildings_yaml(), read_real_recipes_yaml())
        else {
            return; // content/ not present in this checkout layout; skip.
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&byaml).expect("content/base/buildings.yaml must parse");
        let recipes: Vec<crate::content::types::RecipeDef> =
            serde_yaml::from_str(&ryaml).expect("content/base/recipes.yaml must parse");

        let kit: std::collections::BTreeSet<&str> = buildings
            .iter()
            .filter(|b| b.starter_kit)
            .map(|b| b.id.as_str())
            .collect();
        let outputs: std::collections::BTreeSet<&str> = recipes
            .iter()
            .filter(|r| kit.contains(r.building.as_str()))
            .flat_map(|r| r.outputs.iter().map(|o| o.id.as_str()))
            .collect();

        for basic in [
            "housing",
            "oxygen",
            "power",
            "water",
            "food_ration",
            "structural_metal",
            "research",
        ] {
            assert!(
                outputs.contains(basic),
                "no landing-kit building produces {basic}; kit covers {outputs:?}"
            );
        }
    }

    // ── Site preparation (issue #306) ────────────────────────────────────────

    /// The authored slot-granting projects exist, are free of slot cost, and
    /// actually charge materials.
    ///
    /// A slot-granting project that consumed a slot would deadlock a full
    /// colony, and one that cost nothing would make capacity free — the whole
    /// point of #306 is that expansion is bought.
    #[test]
    fn authored_site_preparation_projects_are_slot_free_and_cost_materials() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return; // content/ not present in this checkout layout; skip.
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");

        let granting: Vec<_> = buildings
            .iter()
            .filter(|b| b.grants_slot_capacity > 0)
            .collect();
        assert!(
            granting.len() >= 3,
            "expected the authored site-prep tiers, got {}",
            granting.len()
        );

        for b in &granting {
            assert_eq!(
                b.slot_cost, 0,
                "{} grants slots, so it must not consume one — see #306's deadlock note",
                b.id
            );
            assert!(
                !b.construction_cost.is_empty(),
                "{} must be paid for in materials, not granted free",
                b.id
            );
            assert_eq!(
                b.worker_slots, 0,
                "{} is site preparation, not a staffed building",
                b.id
            );
        }

        // The reverse direction: nothing that is a real building accidentally
        // grants capacity.
        for b in buildings.iter().filter(|b| b.grants_slot_capacity == 0) {
            assert_ne!(
                b.category,
                crate::content::types::BuildingCategory::Infrastructure,
                "{} is categorised as infrastructure but grants no capacity",
                b.id
            );
        }
    }

    /// Larger tiers must buy capacity more cheaply per slot, or there is no
    /// reason to ever research past the first.
    #[test]
    fn later_site_preparation_tiers_are_cheaper_per_slot() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");

        let metal_per_slot = |id: &str| {
            let b = buildings
                .iter()
                .find(|b| b.id == id)
                .unwrap_or_else(|| panic!("{id} must exist in the roster"));
            let metal: f64 = b
                .construction_cost
                .iter()
                .filter(|i| i.id == "structural_metal")
                .map(|i| i.quantity)
                .sum();
            metal / f64::from(b.grants_slot_capacity)
        };

        assert!(
            metal_per_slot("colony_infrastructure") < metal_per_slot("site_preparation"),
            "the mid tier must undercut the starter tier per slot"
        );
        assert!(
            metal_per_slot("arcology_foundation") < metal_per_slot("colony_infrastructure"),
            "the top tier must undercut the mid tier per slot"
        );
    }
}
