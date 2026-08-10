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
use crate::expedition::{AnomalyDef, SurfaceExpeditionFailureDef};

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
        let surface_expedition_failures =
            collect_table::<SurfaceExpeditionFailureDef>(files, &["expedition_failures.yaml"])?;

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

        // Supply packages are physically shipped to a new colony. Most
        // colony-local resources genuinely can't be loaded onto a lander
        // (you can't crate `power`) — but a resource with a storage building
        // in the starter kit (water/`water_tank`, since issue #380) can be,
        // exactly like a tradeable commodity, so this accepts either kind
        // rather than commodities only. A resource seed that lands with
        // nowhere to bank it simply evaporates at the end of the founding
        // sol (see `materialize_founded_colony`'s doc comment) — that's a
        // design decision each such resource's starter-kit storage building
        // needs to account for, not something the loader can enforce.
        for pkg in supply_packages.values() {
            for ing in &pkg.commodities {
                if !producible_ids.contains(ing.id.as_str()) {
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
            surface_expedition_failures,
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

impl HasId for SurfaceExpeditionFailureDef {
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

    /// Read the real `content/base/tech.yaml`, or `None` when the repository
    /// layout doesn't include `content/` beside the crate.
    fn read_real_tech_yaml() -> Option<String> {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
        let root = std::path::Path::new(&manifest).parent()?.to_path_buf();
        std::fs::read_to_string(root.join("content").join("base").join("tech.yaml")).ok()
    }

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

    // ── Contamination remediation (issue #388) ────────────────────────────────

    /// The authored remediation projects are free of slot cost, mirroring
    /// #306's site-preparation guard.
    ///
    /// Unlike `grants_slot_capacity`, the engine does not force
    /// `contamination_reduction`-bearing projects' `slot_cost` to `0` at
    /// queue time — a badly-authored high-`slot_cost` remediation project
    /// really could deadlock a full colony the way #306's engine-level
    /// coercion exists specifically to prevent. This test is the content-side
    /// guard against that class of authoring mistake, so a future
    /// remediation building shipping with a nonzero `slot_cost` fails CI
    /// instead of shipping a real deadlock.
    #[test]
    fn authored_remediation_projects_are_slot_free_and_cost_materials() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return; // content/ not present in this checkout layout; skip.
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");

        let remediating: Vec<_> = buildings
            .iter()
            .filter(|b| b.contamination_reduction > 0.0)
            .collect();
        assert!(
            !remediating.is_empty(),
            "expected at least the authored hex_remediation project"
        );

        for b in &remediating {
            assert_eq!(
                b.slot_cost, 0,
                "{} would deadlock a full colony if it consumed a slot — see #388's note on effective_slot_cost not coercing this the way #306's does",
                b.id
            );
            assert!(
                !b.construction_cost.is_empty(),
                "{} must be paid for in materials, not granted free",
                b.id
            );
            assert_eq!(
                b.worker_slots, 0,
                "{} is a remediation project, not a staffed building",
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

    // ── Surface expedition failure table (issue #340) ────────────────────────

    /// Read the real `content/base/expedition_failures.yaml`, or `None` when
    /// the repository layout doesn't include `content/` beside the crate.
    fn read_real_expedition_failures_yaml() -> Option<String> {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
        let root = std::path::Path::new(&manifest).parent()?.to_path_buf();
        std::fs::read_to_string(
            root.join("content")
                .join("base")
                .join("expedition_failures.yaml"),
        )
        .ok()
    }

    #[test]
    fn real_expedition_failures_yaml_parses_and_has_valid_weights() {
        let Some(yaml) = read_real_expedition_failures_yaml() else {
            return; // content/ not present in this checkout layout; skip.
        };
        let defs: Vec<SurfaceExpeditionFailureDef> =
            serde_yaml::from_str(&yaml).expect("content/base/expedition_failures.yaml must parse");
        assert!(!defs.is_empty());
        for def in &defs {
            assert!(
                (0.0..=1.0).contains(&def.trigger_probability),
                "{} has an out-of-range trigger_probability",
                def.id
            );
            assert!(!def.outcomes.is_empty(), "{} has no outcomes", def.id);
        }
    }

    #[test]
    fn real_expedition_failures_yaml_loads_through_pack_loader() {
        let Some(yaml) = read_real_expedition_failures_yaml() else {
            return;
        };
        let owned = minimal_pack_files(&[("expedition_failures.yaml", yaml.as_str())]);
        let raw: Vec<(&str, &str)> = owned
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        let registry = PackLoader::load(&raw).expect("pack with expedition failures loads");
        assert!(registry
            .surface_expedition_failure("surface_expedition_mishap")
            .is_some());
    }

    // ── Starter power + unique HQ ────────────────────────────────────────────

    /// A colony must be able to add power without researching anything.
    ///
    /// `colony_hq` is capped at one instance, so it can no longer be stacked to
    /// raise the power ceiling — which leaves a colony with no pre-tech route to
    /// more power at all unless a dedicated generator is buildable from
    /// founding. This asserts that route exists rather than trusting the roster
    /// comment, because the two changes only make sense together.
    #[test]
    fn a_dedicated_power_source_is_available_with_no_tech() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return; // content/ not present in this checkout layout; skip.
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");

        let no_tech_generators: Vec<&crate::content::types::BuildingDef> = buildings
            .iter()
            .filter(|b| {
                b.tech_prerequisite.is_none()
                    && b.category == crate::content::types::BuildingCategory::Power
                    // Negative power_delta is the generator convention (see
                    // `compute_power_grid_scaled`).
                    && b.power_delta < 0.0
            })
            .collect();

        assert!(
            !no_tech_generators.is_empty(),
            "no buildable power source exists without tech; colony_hq's cap would \
             leave a colony unable to grow its grid before basic_power"
        );

        // And it must not be capped itself, or it could not scale.
        for gen in &no_tech_generators {
            assert!(
                gen.max_instances.is_none(),
                "{} is the pre-tech power source and must not be limited",
                gen.id
            );
        }
    }

    /// The power roster's tech gates are the ones issue #409 set.
    ///
    /// Pinned individually rather than as a general rule because each was a
    /// distinct authoring error: solar and wind sat behind `basic_power`,
    /// which left a colony with no buildable generator at all on sol 1, and
    /// `fission_reactor` sat behind `improved_solar` — a solar improvement
    /// unlocking a fission reactor.
    #[test]
    fn power_roster_tech_gates_are_as_intended() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        let gate = |id: &str| {
            buildings
                .iter()
                .find(|b| b.id == id)
                .unwrap_or_else(|| panic!("{id} must exist in the roster"))
                .tech_prerequisite
                .clone()
        };

        // Buildable on sol 1.
        assert_eq!(gate("solar_array_mk1"), None);
        assert_eq!(gate("wind_turbine"), None);

        // Still gated, and on nodes that make sense for what they are.
        assert_eq!(
            gate("fission_reactor"),
            Some("nuclear_fuel_cycle".to_string())
        );
        assert_eq!(
            gate("fusion_reactor_prototype"),
            Some("fusion_basics".to_string())
        );
        assert_eq!(gate("solar_array_mk2"), Some("improved_solar".to_string()));
    }

    /// `basic_power` still unlocks something after solar and wind moved off it.
    ///
    /// A tech node that grants nothing is a dead end in the tree — worth
    /// catching here rather than discovering it in play.
    #[test]
    fn basic_power_still_unlocks_a_building() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        let unlocked: Vec<&str> = buildings
            .iter()
            .filter(|b| b.tech_prerequisite.as_deref() == Some("basic_power"))
            .map(|b| b.id.as_str())
            .collect();
        assert!(
            !unlocked.is_empty(),
            "basic_power unlocks nothing now that solar and wind are tech-0"
        );
    }

    /// Each researched generator beats the tier below it, per build slot.
    ///
    /// Slots are the scarce resource in a colony, so a generator is worth what
    /// it produces *per slot* — and on that measure the ladder used to run
    /// backwards (issue #427). `solar_array_mk2` supplied the same grid
    /// capacity as `mk1` and less power for 2.5x the cost, and
    /// `fission_reactor` was worse per slot than two `mk1`s while also costing
    /// a worker, a fuel chain and a waste byproduct.
    ///
    /// Asserts the *ordering* rather than the numbers, so the values stay free
    /// to be retuned by the harness (`docs/DESIGN.md` §17) as long as the
    /// progression survives.
    #[test]
    fn each_power_tier_beats_the_one_below_it_per_slot() {
        let (Some(byaml), Some(ryaml)) = (read_real_buildings_yaml(), read_real_recipes_yaml())
        else {
            return; // content/ not present in this checkout layout; skip.
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&byaml).expect("content/base/buildings.yaml must parse");
        let recipes: Vec<crate::content::types::RecipeDef> =
            serde_yaml::from_str(&ryaml).expect("content/base/recipes.yaml must parse");

        let def = |id: &str| {
            buildings
                .iter()
                .find(|b| b.id == id)
                .unwrap_or_else(|| panic!("{id} must exist in the roster"))
        };
        /// Grid capacity per slot. Negative `power_delta` is the generator
        /// convention (see `compute_power_grid_scaled`).
        let capacity_per_slot = |id: &str| {
            let b = def(id);
            assert!(
                b.slot_cost > 0,
                "{id} occupies no slot; per-slot is meaningless"
            );
            -b.power_delta / f64::from(b.slot_cost)
        };
        let power_per_slot = |id: &str| {
            let produced: f64 = recipes
                .iter()
                .filter(|r| r.building == id)
                .flat_map(|r| r.outputs.iter())
                .filter(|o| o.id == "power")
                .map(|o| o.quantity)
                .sum();
            assert!(produced > 0.0, "{id} produces no power");
            produced / f64::from(def(id).slot_cost)
        };

        // Cheapest to flagship. Each step is a tech tier above the last.
        let ladder = [
            "solar_array_mk1",          // tech 0
            "solar_array_mk2",          // improved_solar
            "fission_reactor",          // nuclear_fuel_cycle
            "fusion_reactor_prototype", // fusion_basics
        ];
        for pair in ladder.windows(2) {
            let (lower, upper) = (pair[0], pair[1]);
            assert!(
                power_per_slot(upper) > power_per_slot(lower),
                "{upper} produces {:.1} power/slot, not more than {lower}'s {:.1} — \
                 researching it would be a downgrade",
                power_per_slot(upper),
                power_per_slot(lower),
            );
            assert!(
                capacity_per_slot(upper) > capacity_per_slot(lower),
                "{upper} supplies {:.1} capacity/slot, not more than {lower}'s {:.1}",
                capacity_per_slot(upper),
                capacity_per_slot(lower),
            );
        }
    }

    /// No generator is dominated by one that is both earlier and cheaper.
    ///
    /// `wind_turbine` is weaker than `solar_array_mk1` per slot and per unit of
    /// metal, but it is also the cheapest generator to put up at all, which is
    /// a real choice for a colony that cannot yet afford solar. Its substantive
    /// differentiation — only working where there is an atmosphere — arrives
    /// with issue #416.
    #[test]
    fn no_generator_is_both_weaker_and_dearer_than_an_untech_gated_one() {
        let (Some(byaml), Some(ryaml)) = (read_real_buildings_yaml(), read_real_recipes_yaml())
        else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&byaml).expect("content/base/buildings.yaml must parse");
        let recipes: Vec<crate::content::types::RecipeDef> =
            serde_yaml::from_str(&ryaml).expect("content/base/recipes.yaml must parse");
        let metal_cost = |b: &crate::content::types::BuildingDef| {
            b.construction_cost
                .iter()
                .filter(|i| i.id == "structural_metal")
                .map(|i| i.quantity)
                .sum::<f64>()
        };

        // Fuel burners are excluded from the comparison, not because they are
        // exempt from balance but because this guard cannot price them. A
        // generator with a perpetual commodity input (and, for
        // combustion_plant, a waste byproduct that contaminates the colony's
        // own hex) is not comparable to an ambient one on construction cost
        // and capacity alone — the whole point of it is that it trades a
        // running cost for reliable output. Comparing them here would either
        // flag every ambient generator as dominated, or force a fuel burner to
        // be artificially weak to keep the test quiet.
        let burns_fuel = |b: &crate::content::types::BuildingDef| {
            recipes
                .iter()
                .any(|r| r.building == b.id && !r.inputs.is_empty())
        };
        let generators: Vec<&crate::content::types::BuildingDef> = buildings
            .iter()
            .filter(|b| {
                matches!(b.category, crate::content::types::BuildingCategory::Power)
                    && b.power_delta < 0.0
                    && !burns_fuel(b)
            })
            .collect();
        assert!(generators.len() >= 4, "expected the full power roster");

        // A site-scaled generator (issue #411/#414/#415) has no single output —
        // its nominal figure is what it would give at a perfect site.
        // Comparing that against a fixed generator would call geothermal
        // dominant on the strength of a hotspot it may never be built on.
        // Compare at an *ordinary* site instead: the question this guard asks
        // is whether a tech-free option makes a researched one pointless in
        // general, and "only where the ground happens to be hot" is a real
        // trade, not dominance.
        //
        // "Ordinary" is per property, not a shared 0.5. The normalised
        // readings are not on a common scale: the geothermal gradient is
        // roughly uniform in [0, 1] so its midpoint is a typical hex, but
        // insolation is normalised *logarithmically* across a 3500-fold range
        // (issue #415), where 0.5 is a body around 4 AU — distant, not
        // typical. Using 0.5 for both would compare a solar array at the
        // outer system against geothermal on average ground.
        let ordinary_reading = |p: &crate::content::types::SiteProperty| match p {
            // 1 AU of a Sol-like star — the point the solar curve is
            // calibrated to produce nominal output at.
            crate::content::types::SiteProperty::Insolation => 0.7927,
            _ => 0.5,
        };
        let effective_capacity_per_slot = |b: &crate::content::types::BuildingDef| {
            let multiplier = b
                .output_scaling
                .as_ref()
                .map_or(1.0, |s| s.multiplier_at(ordinary_reading(&s.property)));
            -b.power_delta * multiplier / f64::from(b.slot_cost)
        };

        for a in &generators {
            for b in &generators {
                if a.id == b.id {
                    continue;
                }
                let a_free = a.tech_prerequisite.is_none();
                let b_gated = b.tech_prerequisite.is_some();
                // Only compare a tech-0 generator against a gated one: two
                // buildings on the same tier are allowed to trade off.
                if !(a_free && b_gated) {
                    continue;
                }
                let dominated = effective_capacity_per_slot(a) >= effective_capacity_per_slot(b)
                    && metal_cost(a) <= metal_cost(b);
                assert!(
                    !dominated,
                    "{} needs tech but is no better per slot than the tech-free {}, \
                     and costs at least as much metal",
                    b.id, a.id
                );
            }
        }
    }

    /// The geothermal plant is buildable at tech 0, and its whole value
    /// comes from the site (issue #414).
    #[test]
    fn geothermal_plant_is_tech_zero_and_site_scaled() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        let plant = buildings
            .iter()
            .find(|b| b.id == "geothermal_plant")
            .expect("geothermal_plant must exist");

        assert_eq!(plant.tech_prerequisite, None, "must be buildable on sol 1");

        let scaling = plant
            .output_scaling
            .as_ref()
            .expect("its output must depend on the site — that is the point of it");
        assert!(
            matches!(
                scaling.property,
                crate::content::types::SiteProperty::GeothermalGradient
            ),
            "must scale on the gradient, not something else"
        );
        assert!(
            scaling.at_max > scaling.at_min * 2.0,
            "the curve is too flat for the site to matter: {} to {}",
            scaling.at_min,
            scaling.at_max
        );
    }

    /// Deep sites need drilling tech, expressed as a *waivable* requirement
    /// rather than a flat `tech_prerequisite` — the site decides whether the
    /// tech is needed at all.
    #[test]
    fn geothermal_plant_gates_deep_sites_on_drilling_tech() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        let plant = buildings
            .iter()
            .find(|b| b.id == "geothermal_plant")
            .expect("geothermal_plant must exist");

        let req = plant
            .site_requirements
            .iter()
            .find(|r| {
                matches!(
                    r.condition,
                    crate::content::types::SiteCondition::MinGeothermalGradient { .. }
                )
            })
            .expect("must carry a minimum-gradient requirement");

        assert_eq!(
            req.waived_by_tech.as_deref(),
            Some("deep_drilling"),
            "a cold site must be reachable *with* drilling tech, not barred outright"
        );
    }

    /// The gate names a tech the tree actually has.
    ///
    /// A typo here would silently make the requirement unwaivable, which
    /// reads in play as a building that can never be placed on cold ground
    /// no matter what is researched.
    #[test]
    fn every_waiving_tech_exists_in_the_tech_tree() {
        let (Some(byaml), Some(tyaml)) = (read_real_buildings_yaml(), read_real_tech_yaml()) else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&byaml).expect("content/base/buildings.yaml must parse");
        let techs: Vec<crate::tech::TechDef> =
            serde_yaml::from_str(&tyaml).expect("content/base/tech.yaml must parse");
        let known: std::collections::HashSet<&str> = techs.iter().map(|t| t.id.as_str()).collect();

        for b in &buildings {
            for r in &b.site_requirements {
                if let Some(tech) = &r.waived_by_tech {
                    assert!(
                        known.contains(tech.as_str()),
                        "{} is waived by unknown tech '{tech}'",
                        b.id
                    );
                }
            }
        }
    }

    /// Wind needs air, and gets better as there is more of it (issue #416).
    #[test]
    fn wind_turbine_requires_an_atmosphere_and_scales_with_it() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        let wind = buildings
            .iter()
            .find(|b| b.id == "wind_turbine")
            .expect("wind_turbine must exist");

        assert_eq!(
            wind.tech_prerequisite, None,
            "still tech 0 after issue #409"
        );

        assert!(
            wind.site_requirements.iter().any(|r| matches!(
                r.condition,
                crate::content::types::SiteCondition::MinAtmosphere { .. }
            )),
            "a turbine on an airless body is nonsense and must be refused"
        );

        let scaling = wind
            .output_scaling
            .as_ref()
            .expect("output must depend on how thick the atmosphere is");
        assert!(
            matches!(
                scaling.property,
                crate::content::types::SiteProperty::AtmosphereDensity
            ),
            "must scale on atmospheric density"
        );
        assert!(
            scaling.at_max > scaling.at_min * 2.0,
            "density barely matters at {} to {}",
            scaling.at_min,
            scaling.at_max
        );
    }

    /// Solar and wind are no longer two identically-shaped buildings where one
    /// simply wins (issues #415/#416).
    ///
    /// Pins that they depend on *different* things, which is what makes the
    /// choice between them a real one rather than a number comparison.
    #[test]
    fn the_two_tech_zero_generators_depend_on_different_site_properties() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        let property_of = |id: &str| {
            buildings
                .iter()
                .find(|b| b.id == id)
                .and_then(|b| b.output_scaling.as_ref())
                .map(|s| s.property.clone())
                .unwrap_or_else(|| panic!("{id} must declare output scaling"))
        };
        assert_ne!(
            property_of("solar_array_mk1"),
            property_of("wind_turbine"),
            "if both scaled on the same property, one would simply dominate"
        );
    }

    /// The whole extract-to-burn chain is reachable at tech 0 (issue #417).
    ///
    /// The failure this guards is subtle: a combustion plant gated at tech 0
    /// is still unbuildable in practice if nothing that produces its fuel is,
    /// and nothing about the plant's own definition would show that.
    #[test]
    fn the_combustion_fuel_chain_closes_at_tech_zero() {
        let (Some(byaml), Some(ryaml)) = (read_real_buildings_yaml(), read_real_recipes_yaml())
        else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&byaml).expect("content/base/buildings.yaml must parse");
        let recipes: Vec<crate::content::types::RecipeDef> =
            serde_yaml::from_str(&ryaml).expect("content/base/recipes.yaml must parse");
        let tech_free: std::collections::HashSet<&str> = buildings
            .iter()
            .filter(|b| b.tech_prerequisite.is_none())
            .map(|b| b.id.as_str())
            .collect();

        assert!(
            tech_free.contains("combustion_plant"),
            "the plant itself must be tech 0"
        );

        // Every commodity the plant burns...
        let burned: Vec<&str> = recipes
            .iter()
            .filter(|r| r.building == "combustion_plant")
            .flat_map(|r| r.inputs.iter())
            .map(|i| i.id.as_str())
            .collect();
        assert!(
            !burned.is_empty(),
            "a combustion plant that burns nothing is not one"
        );

        // ...must be produced by something also reachable at tech 0.
        for fuel in burned {
            let producible = recipes.iter().any(|r| {
                tech_free.contains(r.building.as_str()) && r.outputs.iter().any(|o| o.id == fuel)
            });
            assert!(
                producible,
                "nothing available at tech 0 produces '{fuel}', so the plant cannot \
                 actually be fuelled however its own gate reads"
            );
        }
    }

    /// Burning fuel is not free (issue #417).
    ///
    /// The plant's whole character is a running cost against reliable output;
    /// a byproduct-free version would be strictly better than every ambient
    /// generator with no downside to weigh.
    #[test]
    fn the_combustion_plant_emits_waste_as_well_as_power() {
        let Some(ryaml) = read_real_recipes_yaml() else {
            return;
        };
        let recipes: Vec<crate::content::types::RecipeDef> =
            serde_yaml::from_str(&ryaml).expect("content/base/recipes.yaml must parse");
        let burn = recipes
            .iter()
            .find(|r| r.building == "combustion_plant")
            .expect("the plant must have a recipe");

        assert!(
            burn.outputs
                .iter()
                .any(|o| o.id == "power" && o.quantity > 0.0),
            "must produce power"
        );
        assert!(
            burn.outputs
                .iter()
                .any(|o| o.id == "waste" && o.quantity > 0.0),
            "must emit waste — unhandled waste contaminates the colony's own hex"
        );
    }

    /// Its output does not depend on where it stands — that is the point.
    ///
    /// Every other early generator varies with the site; this one trades that
    /// away for a fuel bill. Adding scaling here would blur the one thing
    /// distinguishing it, and would penalise a poor site twice, since its fuel
    /// is already deposit-gated at the well.
    #[test]
    fn the_combustion_plant_output_is_site_independent() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        let plant = buildings
            .iter()
            .find(|b| b.id == "combustion_plant")
            .expect("combustion_plant must exist");

        assert!(plant.output_scaling.is_none());
        assert!(
            plant.site_requirements.iter().any(|r| matches!(
                &r.condition,
                crate::content::types::SiteCondition::Deposit { commodity, .. }
                    if commodity == "hydrocarbons"
            )),
            "the site decides whether you can have one, not how well it runs"
        );
    }

    /// The marine plants are coastal-only, tech-gated, and genuinely
    /// different from each other (issue #418).
    #[test]
    fn marine_plants_are_coastal_tech_gated_and_not_reskins() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        let plant = |id: &str| {
            buildings
                .iter()
                .find(|b| b.id == id)
                .unwrap_or_else(|| panic!("{id} must exist"))
        };
        let wave = plant("wave_power_plant");
        let thermal = plant("ocean_thermal_plant");
        let current = plant("ocean_current_plant");

        for p in [wave, thermal, current] {
            assert_eq!(
                p.tech_prerequisite.as_deref(),
                Some("marine_power"),
                "{} must sit behind the marine tech",
                p.id
            );
            let coastal = p.site_requirements.iter().any(|r| {
                matches!(
                    &r.condition,
                    crate::content::types::SiteCondition::Terrain { any_of, within_hexes }
                        if any_of.contains(&crate::map::Terrain::Ocean) && *within_hexes == 1
                )
            });
            assert!(
                coastal,
                "{} must require ocean within 1 hex — radius 2 would make \
                 'coastal' mean almost everywhere (measured: ocean is within 1 hex \
                 of a landing site ~51% of the time, within 2 ~83%)",
                p.id
            );
        }

        // The issue's own warning: three ocean plants that differ only in name
        // and cost would be reskins. Each depends on a different thing, so a
        // coastal colony picks whichever its body favours.
        let scaling = |p: &crate::content::types::BuildingDef| {
            p.output_scaling
                .as_ref()
                .unwrap_or_else(|| panic!("{} must scale on something", p.id))
                .clone()
        };
        let props: Vec<_> = [wave, thermal, current].map(|p| scaling(p).property).into();
        for (i, a) in props.iter().enumerate() {
            for b in props.iter().skip(i + 1) {
                assert_ne!(
                    a, b,
                    "each marine plant must depend on a different site property, \
                     or they are reskins with different names and costs"
                );
            }
        }

        // Distinct properties alone would not be enough: three plants with the
        // same curve on three different properties would still feel identical
        // in play. Their *profiles* have to differ too, and current's whole
        // character (issue #440) is that it is the steady one — rotation and
        // tides do not vary the way weather and sunlight do, so it must have
        // the smallest swing and the highest floor of the three.
        let span = |p: &crate::content::types::BuildingDef| {
            let s = scaling(p);
            s.at_max - s.at_min
        };
        let floor = |p: &crate::content::types::BuildingDef| scaling(p).at_min;
        assert!(
            span(current) < span(wave) && span(current) < span(thermal),
            "the current plant must swing least of the three (current {:.2}, \
             wave {:.2}, thermal {:.2}) — steadiness is what distinguishes it",
            span(current),
            span(wave),
            span(thermal)
        );
        assert!(
            floor(current) > floor(wave) && floor(current) > floor(thermal),
            "the current plant must have the best worst-case of the three \
             (current {:.2}, wave {:.2}, thermal {:.2}) — reliable but modest",
            floor(current),
            floor(wave),
            floor(thermal)
        );
    }

    /// A marine plant must beat the unconstrained option of its own tier
    /// where it applies (issue #418).
    ///
    /// `solar_array_mk2` is also tier 2 and buildable anywhere. A plant gated
    /// three ways — tech, a coast, and a body property — that came out weaker
    /// than that would simply never be built, and the test suite would have
    /// been perfectly happy about it.
    #[test]
    fn a_marine_plant_beats_the_unconstrained_option_of_its_tier() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        let cap = |id: &str, reading: f64| {
            let b = buildings
                .iter()
                .find(|x| x.id == id)
                .unwrap_or_else(|| panic!("{id} must exist"));
            let m = b
                .output_scaling
                .as_ref()
                .map_or(1.0, |s| s.multiplier_at(reading));
            -b.power_delta * m / f64::from(b.slot_cost)
        };

        // mk2 at its own calibration point (1 AU).
        let mk2 = cap("solar_array_mk2", 0.7927);
        // Wave on a breathable atmosphere; thermal at 1 AU.
        assert!(
            cap("wave_power_plant", 0.667) > mk2,
            "wave on a breathable world gives {:.1} against mk2's {mk2:.1}",
            cap("wave_power_plant", 0.667)
        );
        assert!(
            cap("ocean_thermal_plant", 0.7927) > mk2,
            "thermal at 1 AU gives {:.1} against mk2's {mk2:.1}",
            cap("ocean_thermal_plant", 0.7927)
        );
        // Current at the median *applicable* site (issue #440). Moons are
        // 85.5% of foundable bodies and their median ocean_circulation is
        // 0.44, so that — not the whole-population median — is the reading
        // this plant has to clear mk2 at.
        assert!(
            cap("ocean_current_plant", 0.442) > mk2,
            "current on a median moon gives {:.1} against mk2's {mk2:.1}",
            cap("ocean_current_plant", 0.442)
        );
        // And it must genuinely be a poor choice on a dead sea, or the site
        // property is decoration. A tidally locked planet reads ~0.013.
        assert!(
            cap("ocean_current_plant", 0.013) < mk2,
            "current on a tidally locked planet gives {:.1}, which should be \
             below mk2's {mk2:.1} — a dead sea has to be a bad site",
            cap("ocean_current_plant", 0.013)
        );
    }

    /// A colony gets exactly one headquarters.
    #[test]
    fn colony_hq_is_capped_at_one_instance() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        let hq = buildings
            .iter()
            .find(|b| b.id == "colony_hq")
            .expect("colony_hq must exist in the roster");
        assert_eq!(hq.max_instances, Some(1));
    }

    /// Nothing else is capped by accident.
    ///
    /// `max_instances` silently truncates what a player can build, so a stray
    /// one on an ordinary utility building would read in play as a bug. Pinning
    /// the set means adding another cap is a deliberate edit here too.
    #[test]
    fn only_the_headquarters_is_capped() {
        let Some(yaml) = read_real_buildings_yaml() else {
            return;
        };
        let buildings: Vec<crate::content::types::BuildingDef> =
            serde_yaml::from_str(&yaml).expect("content/base/buildings.yaml must parse");
        let capped: Vec<&str> = buildings
            .iter()
            .filter(|b| b.max_instances.is_some())
            .map(|b| b.id.as_str())
            .collect();
        assert_eq!(capped, vec!["colony_hq"]);
    }
}
