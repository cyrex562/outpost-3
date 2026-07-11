//! In-memory content registry: queryable typed tables populated by the loader.

use std::collections::HashMap;

use super::types::{
    BuildingDef, CommodityDef, DefaultDirectiveDef, OrbitalStationBlueprint, PackManifest,
    RecipeDef, SupplyPackage,
};

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
}

impl ContentRegistry {
    /// Merge `other` into `self`.  On ID collision, `other` wins (later load wins).
    pub fn merge(&mut self, other: ContentRegistry) {
        self.manifest = other.manifest;
        self.commodities.extend(other.commodities);
        self.recipes.extend(other.recipes);
        self.buildings.extend(other.buildings);
        self.orbital_blueprints.extend(other.orbital_blueprints);
        self.default_directives.extend(other.default_directives);
        self.supply_packages.extend(other.supply_packages);
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

    /// Insert or replace a building definition (used in tests and harness tooling).
    pub fn insert_building(&mut self, def: BuildingDef) {
        self.buildings.insert(def.id.clone(), def);
    }

    /// Insert or replace a recipe definition (used in tests and harness tooling).
    pub fn insert_recipe(&mut self, def: RecipeDef) {
        self.recipes.insert(def.id.clone(), def);
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
