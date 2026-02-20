//! Storage and production rate helpers for the server layer.
//!
//! These functions compute derived data from site + building definitions
//! that is too expensive to bake into the domain layer itself (because
//! the domain doesn't own the definitions HashMap).

use crate::content::{BuildingDefinition, BuildingCategory, ContentLoader};
use crate::domain::Site;
use crate::domain::resource_mapping::is_virtual_resource;
use std::collections::HashMap;


// ──────────────────────────────────────────────────────────────────────────
// Storage Capacity
// ──────────────────────────────────────────────────────────────────────────

/// Base storage built into every settlement (before warehouses are built).
const BASE_STORAGE_CAPACITY: f64 = 5_000.0;

/// Compute the total storage capacity for a site by aggregating the
/// `storage_capacity` field from all **operational** Storage-category buildings.
///
/// Returns `BASE_STORAGE_CAPACITY + Σ(operational warehouse storage)`.
pub fn compute_site_storage(
    site: &Site,
    defs: &HashMap<String, BuildingDefinition>,
) -> f64 {
    let warehouse_total: f64 = site.buildings.values()
        .filter(|b| b.state.can_operate())
        .filter_map(|b| {
            let def = defs.get(&b.building_def_id)?;
            if def.category == BuildingCategory::Storage {
                def.storage_capacity
            } else {
                None
            }
        })
        .sum();
    BASE_STORAGE_CAPACITY + warehouse_total
}

/// Compute per-resource storage utilisation percentage for a site.
///
/// Returns a map from resource name → (current, capacity, percent).
pub fn compute_storage_utilisation(
    site: &Site,
    defs: &HashMap<String, BuildingDefinition>,
) -> HashMap<String, (f64, f64, f64)> {
    let capacity = compute_site_storage(site, defs);
    let mut result = HashMap::new();

    for (resource_type, quantity) in site.resources.iter() {
        let current = *quantity as f64;
        let percent = (current / capacity * 100.0).clamp(0.0, 100.0);
        result.insert(resource_type.name().to_string(), (current, capacity, percent));
    }
    result
}

// ──────────────────────────────────────────────────────────────────────────
// Production / Consumption Rates
// ──────────────────────────────────────────────────────────────────────────

/// Per-tick production and consumption rates for a resource at a site.
#[derive(Debug, Clone, Default)]
pub struct ResourceRate {
    /// Units produced per tick from all buildings with active recipes.
    pub production: f64,
    /// Units consumed per tick from all buildings with active recipes.
    pub consumption: f64,
}

impl ResourceRate {
    pub fn net(&self) -> f64 {
        self.production - self.consumption
    }
}

/// Compute per-tick production and consumption rates for every resource at a site.
///
/// Derived from the **active recipe** of each operational building. Virtual resources
/// (power, labor) are excluded from the rates map.
///
/// # Returns
/// Map from resource string-id → `ResourceRate`.
pub fn compute_resource_rates(
    site: &Site,
    defs: &HashMap<String, BuildingDefinition>,
    content: &ContentLoader,
) -> HashMap<String, ResourceRate> {
    let mut rates: HashMap<String, ResourceRate> = HashMap::new();

    for building in site.buildings.values() {
        if !building.state.can_operate() {
            continue;
        }

        let recipe_id = match building.get_recipe_id() {
            Some(id) => id,
            None => continue,
        };

        let recipe = match content.get_recipe(recipe_id) {
            Some(r) => r,
            None => continue,
        };

        let processing_ticks = recipe.processing_time_ticks.max(1) as f64;

        // Consumption — inputs
        for (resource_id, amount) in &recipe.inputs {
            if is_virtual_resource(resource_id) {
                continue;
            }
            let rate = rates.entry(resource_id.clone()).or_default();
            // Inputs consumed once per cycle, rate = amount / processing_ticks
            rate.consumption += amount / processing_ticks;
        }

        // Production — outputs
        for (resource_id, amount) in &recipe.outputs {
            if is_virtual_resource(resource_id) {
                continue;
            }
            let rate = rates.entry(resource_id.clone()).or_default();
            rate.production += amount / processing_ticks;
        }
    }

    rates
}

// ──────────────────────────────────────────────────────────────────────────
// Tooltip Helpers — Producers and Consumers
// ──────────────────────────────────────────────────────────────────────────

/// For each resource, list the names of building definitions whose recipes
/// **produce** that resource (used for tooltip overlays).
///
/// # Returns
/// Map from resource string-id → list of building display names.
pub fn list_resource_producers(
    defs: &HashMap<String, BuildingDefinition>,
    content: &ContentLoader,
) -> HashMap<String, Vec<String>> {
    let mut producers: HashMap<String, Vec<String>> = HashMap::new();

    for def in defs.values() {
        // Check both inline recipes (from BuildingDefinition) and global recipes
        // from content that reference this building type.
        let relevant_recipes: Vec<_> = content.all_recipes().values()
            .filter(|r| r.building_types.contains(&def.id))
            .collect();

        for recipe in &relevant_recipes {
            for resource_id in recipe.outputs.keys() {
                if !is_virtual_resource(resource_id) {
                    producers
                        .entry(resource_id.clone())
                        .or_default()
                        .push(def.name.clone());
                }
            }
        }
    }

    // Deduplicate
    for names in producers.values_mut() {
        names.dedup();
    }
    producers
}

/// For each resource, list the names of building definitions whose recipes
/// **consume** that resource (used for tooltip overlays).
pub fn list_resource_consumers(
    defs: &HashMap<String, BuildingDefinition>,
    content: &ContentLoader,
) -> HashMap<String, Vec<String>> {
    let mut consumers: HashMap<String, Vec<String>> = HashMap::new();

    for def in defs.values() {
        let relevant_recipes: Vec<_> = content.all_recipes().values()
            .filter(|r| r.building_types.contains(&def.id))
            .collect();

        for recipe in &relevant_recipes {
            for resource_id in recipe.inputs.keys() {
                if !is_virtual_resource(resource_id) {
                    consumers
                        .entry(resource_id.clone())
                        .or_default()
                        .push(def.name.clone());
                }
            }
        }
    }

    for names in consumers.values_mut() {
        names.dedup();
    }
    consumers
}

// ──────────────────────────────────────────────────────────────────────────
// Active Production Chains Summary
// ──────────────────────────────────────────────────────────────────────────

/// Summary of a single active production chain for display in the UI.
#[derive(Debug, Clone)]
pub struct ActiveChain {
    pub building_name: String,
    pub recipe_name: String,
    pub progress_pct: u32,
    pub inputs: HashMap<String, f64>,
    pub outputs: HashMap<String, f64>,
}

/// Gather active production chains for all operational buildings at a site.
pub fn active_production_chains(
    site: &Site,
    defs: &HashMap<String, BuildingDefinition>,
    content: &ContentLoader,
) -> Vec<ActiveChain> {
    let mut chains = Vec::new();

    for building in site.buildings.values() {
        if !building.state.can_operate() {
            continue;
        }
        let recipe_id = match building.get_recipe_id() {
            Some(id) => id,
            None => continue,
        };
        let recipe = match content.get_recipe(recipe_id) {
            Some(r) => r,
            None => continue,
        };
        let def_name = defs.get(&building.building_def_id)
            .map(|d| d.name.as_str())
            .unwrap_or(&building.building_def_id)
            .to_string();

        let total_ticks = recipe.processing_time_ticks.max(1);
        let progress_pct = ((building.recipe_progress_ticks as f64 / total_ticks as f64) * 100.0)
            .round() as u32;

        chains.push(ActiveChain {
            building_name: def_name,
            recipe_name: recipe.name.clone(),
            progress_pct,
            inputs: recipe.inputs.iter()
                .filter(|(id, _)| !is_virtual_resource(id))
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
            outputs: recipe.outputs.iter()
                .filter(|(id, _)| !is_virtual_resource(id))
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
        });
    }

    chains.sort_by(|a, b| a.building_name.cmp(&b.building_name));
    chains
}

// ──────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CelestialBodyId, Site};
    use crate::domain::building_instance::BuildingInstance;
    use crate::content::{BuildingDefinition, BuildingCategory};

    fn make_warehouse_def(capacity: f64) -> BuildingDefinition {
        BuildingDefinition {
            id: "warehouse".to_string(),
            name: "Warehouse".to_string(),
            description: "Storage".to_string(),
            category: BuildingCategory::Storage,
            construction_costs: vec![],
            construction_time_ticks: 100,
            power: None,
            power_output_mw: None,
            worker_capacity: 2,
            storage_capacity: Some(capacity),
            housing_capacity: None,
            recipes: vec![],
            maintenance_cost: HashMap::new(),
            upgradeable: false,
            upgrade_to: None,
            morale_bonus: 0.0,
        }
    }

    fn make_site() -> Site {
        Site::new_settlement(CelestialBodyId::new(), "Test".to_string(), 0, 50)
    }

    #[test]
    fn test_base_storage_with_no_buildings() {
        let site = make_site();
        let defs = HashMap::new();
        let capacity = compute_site_storage(&site, &defs);
        assert_eq!(capacity, BASE_STORAGE_CAPACITY);
    }

    #[test]
    fn test_storage_aggregates_warehouses() {
        let mut site = make_site();
        let mut defs = HashMap::new();
        defs.insert("warehouse".to_string(), make_warehouse_def(10_000.0));

        // Add an operational warehouse
        use crate::domain::SiteId;
        let building = BuildingInstance::new_operational(
            site.id,
            "warehouse".to_string(),
            0,
        );
        site.buildings.insert(building.id, building);

        let capacity = compute_site_storage(&site, &defs);
        assert_eq!(capacity, BASE_STORAGE_CAPACITY + 10_000.0);
    }

    #[test]
    fn test_storage_ignores_non_storage_buildings() {
        let mut site = make_site();
        let mut defs = HashMap::new();
        // A smelter with a non-None storage_capacity (edge case)
        defs.insert("smelter".to_string(), BuildingDefinition {
            id: "smelter".to_string(),
            name: "Smelter".to_string(),
            description: "".to_string(),
            category: BuildingCategory::Refining,  // NOT Storage
            construction_costs: vec![],
            construction_time_ticks: 100,
            power: None, power_output_mw: None,
            worker_capacity: 8,
            storage_capacity: Some(99_999.0), // Should be ignored
            housing_capacity: None,
            recipes: vec![],
            maintenance_cost: HashMap::new(),
            upgradeable: false,
            upgrade_to: None,
            morale_bonus: 0.0,
        });

        use crate::domain::SiteId;
        let building = BuildingInstance::new_operational(site.id, "smelter".to_string(), 0);
        site.buildings.insert(building.id, building);

        let capacity = compute_site_storage(&site, &defs);
        assert_eq!(capacity, BASE_STORAGE_CAPACITY); // Smelter not counted
    }

    #[test]
    fn test_storage_ignores_non_operational_warehouses() {
        let mut site = make_site();
        let mut defs = HashMap::new();
        defs.insert("warehouse".to_string(), make_warehouse_def(10_000.0));

        use crate::domain::SiteId;
        // A warehouse under construction — NOT operational
        let mut building = BuildingInstance::new(
            site.id,
            "warehouse".to_string(),
            0,
        );
        // building is UnderConstruction by default
        site.buildings.insert(building.id, building);

        let capacity = compute_site_storage(&site, &defs);
        assert_eq!(capacity, BASE_STORAGE_CAPACITY); // Not counting under-construction
    }

    #[test]
    fn test_compute_resource_rates_idle_buildings() {
        let site = make_site();
        let defs = HashMap::new();
        let content = ContentLoader::new();

        let rates = compute_resource_rates(&site, &defs, &content);
        assert!(rates.is_empty(), "No buildings → no rates");
    }

    #[test]
    fn test_compute_resource_rates_with_recipe() {
        let mut site = make_site();
        let defs = HashMap::new();

        let recipe_yaml = r#"
recipes:
  - id: smelt_iron
    name: "Smelt Iron"
    building_types: [smelter]
    inputs:
      iron_ore: 15
      power: 20
    outputs:
      iron_ingots: 10
    processing_time_ticks: 5
"#;
        let mut content = ContentLoader::new();
        content.load_recipes(recipe_yaml).expect("load recipes");

        use crate::domain::SiteId;
        let mut building = BuildingInstance::new_operational(site.id, "smelter".to_string(), 0);
        building.set_recipe("smelt_iron".to_string());
        site.buildings.insert(building.id, building);

        let rates = compute_resource_rates(&site, &defs, &content);

        // iron_ore consumed at 15/5 = 3.0 per tick
        assert!(rates.contains_key("iron_ore"));
        assert!((rates["iron_ore"].consumption - 3.0).abs() < 0.01);

        // iron_ingots produced at 10/5 = 2.0 per tick
        assert!(rates.contains_key("iron_ingots"));
        assert!((rates["iron_ingots"].production - 2.0).abs() < 0.01);

        // power is virtual — should NOT appear
        assert!(!rates.contains_key("power"));
    }

    #[test]
    fn test_resource_rate_net() {
        let rate = ResourceRate { production: 10.0, consumption: 3.0 };
        assert!((rate.net() - 7.0).abs() < 0.01);
    }
}
