//! In-memory content registry: queryable typed tables populated by the loader.

use std::collections::HashMap;

use super::types::{
    BuildingDef, CommodityDef, DefaultDirectiveDef, OrbitalStationBlueprint, PackManifest,
    RecipeDef, ResourceDef, StarSystemDef, SupplyPackage,
};
use crate::expedition::AnomalyDef;

/// In-memory registry produced by loading one or more content packs.
///
/// Queryable by typed accessor methods.  Merge multiple packs with
/// [`ContentRegistry::merge`] — later wins on ID collision.
#[derive(Debug, Default, Clone)]
pub struct ContentRegistry {
    /// Manifest of the last pack merged (or the only pack).
    pub(super) manifest: PackManifest,
    /// All commodity definitions, keyed by id.
    pub(super) commodities: HashMap<String, CommodityDef>,
    /// All colony-resource definitions, keyed by id (issue #304). Disjoint from
    /// `commodities`: an id is either tradeable cargo or a colony-local
    /// resource, never both.
    pub(super) resources: HashMap<String, ResourceDef>,
    /// All recipe definitions, keyed by id.
    pub(super) recipes: HashMap<String, RecipeDef>,
    /// All building definitions, keyed by id.
    pub(super) buildings: HashMap<String, BuildingDef>,
    /// All orbital station blueprints, keyed by id.
    pub orbital_blueprints: HashMap<String, OrbitalStationBlueprint>,
    /// Default directives inserted into every newly-founded colony.
    pub default_directives: Vec<DefaultDirectiveDef>,
    /// Named starter-supply packages selectable at colony founding.
    pub(super) supply_packages: HashMap<String, SupplyPackage>,
    /// Authored star-system scenarios used to seed `SystemState.node_map`.
    pub(super) star_systems: HashMap<String, StarSystemDef>,
    /// Anomaly definitions discoverable during survey expeditions (issue #235).
    pub(super) anomalies: HashMap<String, AnomalyDef>,
}

impl ContentRegistry {
    /// Merge `other` into `self`.  On ID collision, `other` wins (later load wins).
    pub fn merge(&mut self, other: ContentRegistry) {
        self.manifest = other.manifest;
        self.commodities.extend(other.commodities);
        self.resources.extend(other.resources);
        self.recipes.extend(other.recipes);
        self.buildings.extend(other.buildings);
        self.orbital_blueprints.extend(other.orbital_blueprints);
        self.default_directives.extend(other.default_directives);
        self.supply_packages.extend(other.supply_packages);
        self.star_systems.extend(other.star_systems);
        self.anomalies.extend(other.anomalies);
    }

    /// The manifest of the most-recently loaded pack.
    #[must_use]
    pub fn manifest(&self) -> &PackManifest {
        &self.manifest
    }

    /// Look up a commodity by id.
    #[must_use]
    pub fn commodity(&self, id: &str) -> Option<&CommodityDef> {
        self.commodities.get(id)
    }

    /// Look up a colony resource by id (issue #304).
    #[must_use]
    pub fn resource(&self, id: &str) -> Option<&ResourceDef> {
        self.resources.get(id)
    }

    /// All colony resources as an iterator.
    pub fn resources(&self) -> impl Iterator<Item = &ResourceDef> {
        self.resources.values()
    }

    /// Whether `id` names a colony-local resource rather than a tradeable
    /// commodity. This is the single dispatch point deciding which pool an
    /// ingredient or need reads from.
    #[must_use]
    pub fn is_resource(&self, id: &str) -> bool {
        self.resources.contains_key(id)
    }

    /// All commodities as an iterator.
    pub fn commodities(&self) -> impl Iterator<Item = &CommodityDef> {
        self.commodities.values()
    }

    /// Look up a recipe by id.
    #[must_use]
    pub fn recipe(&self, id: &str) -> Option<&RecipeDef> {
        self.recipes.get(id)
    }

    /// All recipes as an iterator.
    pub fn recipes(&self) -> impl Iterator<Item = &RecipeDef> {
        self.recipes.values()
    }

    /// Look up a building by id.
    #[must_use]
    pub fn building(&self, id: &str) -> Option<&BuildingDef> {
        self.buildings.get(id)
    }

    /// All buildings as an iterator.
    pub fn buildings(&self) -> impl Iterator<Item = &BuildingDef> {
        self.buildings.values()
    }

    /// The landing kit: buildings every colony starts with, id-sorted
    /// (issue #317).
    ///
    /// Sorted so the placed instances come out in a stable order — buildings are
    /// stored in a `HashMap`, and an arbitrary order would make the starting
    /// colony's building list differ between runs of the same seed.
    ///
    /// Empty when no pack marks anything, which keeps every existing test and
    /// harness fixture behaving as it did.
    #[must_use]
    pub fn starter_kit(&self) -> Vec<&BuildingDef> {
        let mut kit: Vec<&BuildingDef> =
            self.buildings.values().filter(|b| b.starter_kit).collect();
        kit.sort_by(|a, b| a.id.cmp(&b.id));
        kit
    }

    /// Insert or replace a building definition (used in tests and harness tooling).
    pub fn insert_building(&mut self, def: BuildingDef) {
        self.buildings.insert(def.id.clone(), def);
    }

    /// Insert or replace a recipe definition (used in tests and harness tooling).
    pub fn insert_recipe(&mut self, def: RecipeDef) {
        self.recipes.insert(def.id.clone(), def);
    }

    /// Insert or replace a colony-resource definition (tests and harness tooling).
    pub fn insert_resource(&mut self, def: super::types::ResourceDef) {
        self.resources.insert(def.id.clone(), def);
    }

    /// Insert or replace a commodity definition (used in tests and harness tooling).
    pub fn insert_commodity(&mut self, def: super::types::CommodityDef) {
        self.commodities.insert(def.id.clone(), def);
    }

    /// Insert or replace an orbital station blueprint (used in tests and harness tooling).
    pub fn insert_orbital_blueprint(&mut self, def: OrbitalStationBlueprint) {
        self.orbital_blueprints.insert(def.id.clone(), def);
    }

    /// All default directives for newly-founded colonies.
    #[must_use]
    pub fn default_directives(&self) -> &[DefaultDirectiveDef] {
        &self.default_directives
    }

    /// Look up a supply package by id.
    #[must_use]
    pub fn supply_package(&self, id: &str) -> Option<&SupplyPackage> {
        self.supply_packages.get(id)
    }

    /// All supply packages as an iterator.
    pub fn supply_packages(&self) -> impl Iterator<Item = &SupplyPackage> {
        self.supply_packages.values()
    }

    /// Look up a star-system scenario by id.
    #[must_use]
    pub fn star_system(&self, id: &str) -> Option<&StarSystemDef> {
        self.star_systems.get(id)
    }

    /// All star-system scenarios as an iterator.
    pub fn star_systems(&self) -> impl Iterator<Item = &StarSystemDef> {
        self.star_systems.values()
    }

    /// Look up an anomaly by id.
    #[must_use]
    pub fn anomaly(&self, id: &str) -> Option<&AnomalyDef> {
        self.anomalies.get(id)
    }

    /// All anomaly definitions as an iterator.
    pub fn anomalies(&self) -> impl Iterator<Item = &AnomalyDef> {
        self.anomalies.values()
    }

    /// Insert or replace an anomaly definition (used in tests and harness tooling).
    pub fn insert_anomaly(&mut self, def: AnomalyDef) {
        self.anomalies.insert(def.id.clone(), def);
    }
}

// Default manifest used by Default derive.
impl Default for PackManifest {
    fn default() -> Self {
        PackManifest {
            id: String::new(),
            name: String::new(),
            version: String::from("0.0.0"),
            description: String::new(),
        }
    }
}
