//! Production simulation — per-tick recipe execution for operational buildings.
//!
//! This module handles the core production logic:
//! - Validate recipe prerequisites (inputs, power, labor, deposits)
//! - Consume input resources from site stockpile
//! - Track recipe progress across ticks
//! - Produce output resources when recipe completes
//! - Handle storage capacity constraints

use crate::content::ContentLoader;
use crate::domain::{
    BuildingInstance, Recipe, RecipeError, Site, BuildingId, CelestialBody, MappingError,
    is_virtual_resource,
};
use crate::simulation::storage_helpers::compute_site_storage;
use crate::simulation::power_processor::building_power_efficiency;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during production processing
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ProductionError {
    #[error("Building {0} is not operational")]
    BuildingNotOperational(BuildingId),

    #[error("No recipe assigned to building {0}")]
    NoRecipeAssigned(BuildingId),

    #[error("Recipe error: {0}")]
    RecipeError(#[from] RecipeError),

    #[error("Resource mapping error: {0}")]
    MappingError(#[from] MappingError),

    #[error("Insufficient resource in stockpile: {resource}, need {needed}, have {available}")]
    InsufficientResource {
        resource: String,
        needed: f64,
        available: f64,
    },

    #[error("Insufficient storage for output: {resource}, would produce {amount}, only {available} space")]
    InsufficientStorage {
        resource: String,
        amount: f64,
        available: f64,
    },

    #[error("Required deposit {resource} not found or depleted on body")]
    MissingOrDepletedDeposit { resource: String },
}

/// Result of processing a single building's production for one tick
#[derive(Debug, Clone)]
pub enum ProductionResult {
    /// Recipe is in progress (not yet complete)
    InProgress {
        building_id: BuildingId,
        recipe_id: String,
        progress_ticks: u64,
        processing_time_ticks: u64,
    },

    /// Recipe completed this tick
    Completed {
        building_id: BuildingId,
        recipe_id: String,
        inputs_consumed: HashMap<String, f64>,
        outputs_produced: HashMap<String, f64>,
    },

    /// Production halted due to constraints
    Halted {
        building_id: BuildingId,
        recipe_id: String,
        reason: ProductionError,
    },

    /// Building has no recipe or is not operational
    Idle {
        building_id: BuildingId,
    },
}

/// Process one building's production for one tick.
///
/// This function:
/// 1. Checks if building is operational and has a recipe
/// 2. Validates inputs are available in stockpile
/// 3. Validates outputs have storage space
/// 4. For extraction recipes, validates deposit exists and has resources
/// 5. Consumes inputs when starting a recipe cycle (progress = 0)
/// 6. Advances recipe progress by 1 tick
/// 7. Produces outputs when recipe completes
///
/// Returns ProductionResult indicating what happened.
pub fn process_building_production(
    building_id: BuildingId,
    site: &mut Site,
    body: &mut CelestialBody,
    content: &ContentLoader,
) -> Result<ProductionResult, ProductionError> {
    // Get the building (borrowing site.buildings temporarily)
    let (can_operate, recipe_id_opt, progress_ticks, workers_assigned, building_def_id, labor_accumulator) = {
        let building = match site.buildings.get(&building_id) {
            Some(b) => b,
            None => return Ok(ProductionResult::Idle { building_id }),
        };
        
        (
            building.state.can_operate(),
            building.get_recipe_id().map(|s| s.to_string()),
            building.recipe_progress_ticks,
            building.workers_assigned,
            building.building_def_id.clone(),
            building.labor_progress_accumulator,
        )
    };
    
    // Check if building can produce
    if !can_operate {
        return Ok(ProductionResult::Idle { building_id });
    }

    // Check if building has a recipe assigned
    let recipe_id = match recipe_id_opt {
        Some(id) => id,
        None => return Ok(ProductionResult::Idle { building_id }),
    };

    // Compute labor efficiency (worker_capacity 0 = no labor required → 100% efficient)
    let worker_capacity = content
        .all_buildings()
        .get(&building_def_id)
        .map(|def| def.worker_capacity)
        .unwrap_or(0);
    let staffing_ratio = if worker_capacity == 0 {
        1.0_f64
    } else {
        (workers_assigned as f64 / worker_capacity as f64).clamp(0.0, 1.0)
    };

    // Morale modifier: high morale boosts efficiency, low morale reduces it.
    // morale=100 → ×1.2, morale=40 → ×1.0, morale=0 → ×0.7
    let morale = site.population.morale as f64;
    let morale_modifier = if morale >= 40.0 {
        1.0 + (morale - 40.0) / 60.0 * 0.2
    } else {
        1.0 - (40.0 - morale) / 40.0 * 0.3
    }
    .clamp(0.7, 1.2);

    let labor_efficiency = (staffing_ratio * morale_modifier).clamp(0.0, 1.0);

    // Power brownout modifier: essential buildings are unaffected, others are penalized.
    let power_efficiency = building_power_efficiency(
        &building_def_id,
        site.power_efficiency,
        content.all_buildings(),
    );

    let total_efficiency = (labor_efficiency * power_efficiency).clamp(0.0, 1.0);

    // No workers at all for a building that requires them → halt
    if worker_capacity > 0 && workers_assigned == 0 {
        return Ok(ProductionResult::Halted {
            building_id,
            recipe_id: recipe_id.clone(),
            reason: ProductionError::InsufficientResource {
                resource: "labor".to_string(),
                needed: worker_capacity as f64,
                available: 0.0,
            },
        });
    }

    // Get the recipe definition
    let recipe = content
        .get_recipe(&recipe_id)
        .ok_or_else(|| ProductionError::RecipeError(RecipeError::RecipeNotFound(recipe_id.clone())))?;

    // If this is the start of a new cycle (progress = 0 and accumulator = 0),
    // validate and consume inputs. The accumulator > 0 means we've already started.
    if progress_ticks == 0 && labor_accumulator == 0 {
        // First check if we CAN start (validation phase)
        if let Err(e) = validate_production_start(site, body, recipe) {
            return Ok(ProductionResult::Halted {
                building_id,
                recipe_id: recipe.id.clone(),
                reason: e,
            });
        }

        // All valid, consume inputs
        consume_inputs(site, body, recipe)?;
    }

    // Advance recipe progress scaled by total efficiency (labor × morale × power).
    let completed = {
        let building = site.buildings.get_mut(&building_id).unwrap();
        building.advance_recipe_progress_with_efficiency(recipe.processing_time_ticks, total_efficiency)
    };

    if completed {
        // Recipe completed! Check if we can produce outputs
        if let Err(e) = validate_output_storage(site, recipe, content) {
            // Storage full - halt production
            // Progress was reset, so next tick will try to start a new cycle
            return Ok(ProductionResult::Halted {
                building_id,
                recipe_id: recipe.id.clone(),
                reason: e,
            });
        }

        // Produce outputs
        let outputs = produce_outputs(site, recipe)?;

        Ok(ProductionResult::Completed {
            building_id,
            recipe_id: recipe.id.clone(),
            inputs_consumed: recipe.inputs.clone(),
            outputs_produced: outputs,
        })
    } else {
        // Still in progress - get current progress
        let current_progress = site.buildings.get(&building_id).unwrap().recipe_progress_ticks;
        
        Ok(ProductionResult::InProgress {
            building_id,
            recipe_id: recipe.id.clone(),
            progress_ticks: current_progress,
            processing_time_ticks: recipe.processing_time_ticks,
        })
    }
}

/// Validate that production can start (all inputs available, deposit present if needed).
fn validate_production_start(
    site: &Site,
    body: &CelestialBody,
    recipe: &Recipe,
) -> Result<(), ProductionError> {
    // Check all physical inputs are available
    for (resource_id, quantity) in &recipe.inputs {
        // Skip virtual resources (power, labor)
        if is_virtual_resource(resource_id) {
            continue;
        }

        let available = site.get_resource_by_id(resource_id)?;

        if available < *quantity {
            return Err(ProductionError::InsufficientResource {
                resource: resource_id.clone(),
                needed: *quantity,
                available,
            });
        }
    }

    // For extraction recipes, validate deposit exists and has resources
    if let Some(deposit_resource) = recipe.required_deposit_resource() {
        let deposits = body.get_deposits(deposit_resource);
        let total_remaining: f64 = deposits.iter().map(|d| d.remaining_quantity).sum();

        if total_remaining == 0.0 {
            return Err(ProductionError::MissingOrDepletedDeposit {
                resource: deposit_resource.to_string(),
            });
        }
    }

    Ok(())
}

/// Consume inputs from stockpile and deplete deposits if needed.
fn consume_inputs(
    site: &mut Site,
    body: &mut CelestialBody,
    recipe: &Recipe,
) -> Result<(), ProductionError> {
    // Consume physical inputs from stockpile
    for (resource_id, quantity) in &recipe.inputs {
        if is_virtual_resource(resource_id) {
            continue; // Don't consume virtual resources from stockpile
        }

        site.adjust_resource_by_id(resource_id, -*quantity)?;
    }

    // For extraction recipes, deplete deposit
    if let Some(deposit_resource) = recipe.required_deposit_resource() {
        // Find the first non-depleted deposit and extract from it
        let output_quantity = recipe
            .outputs
            .get(deposit_resource)
            .copied()
            .unwrap_or(0.0);

        let mut deposits = body.get_deposits_mut(deposit_resource);
        if let Some(deposit) = deposits.iter_mut().find(|d| !d.is_depleted()) {
            deposit.extract(output_quantity);
        }
    }

    Ok(())
}

/// Validate that outputs can fit in storage.
fn validate_output_storage(
    site: &Site,
    recipe: &Recipe,
    content: &ContentLoader,
) -> Result<(), ProductionError> {
    let capacity = compute_site_storage(site, content.all_buildings());
    for (resource_id, quantity) in &recipe.outputs {
        if is_virtual_resource(resource_id) {
            continue; // Virtual resources don't need storage
        }

        let current = site.get_resource_by_id(resource_id)?;
        let available_space = capacity - current;

        if available_space < *quantity {
            return Err(ProductionError::InsufficientStorage {
                resource: resource_id.clone(),
                amount: *quantity,
                available: available_space,
            });
        }
    }

    Ok(())
}

/// Produce outputs and add them to site stockpile.
fn produce_outputs(
    site: &mut Site,
    recipe: &Recipe,
) -> Result<HashMap<String, f64>, ProductionError> {
    let mut produced = HashMap::new();

    for (resource_id, quantity) in &recipe.outputs {
        if is_virtual_resource(resource_id) {
            continue; // Don't add virtual resources to stockpile
        }

        site.adjust_resource_by_id(resource_id, *quantity)?;
        produced.insert(resource_id.clone(), *quantity);
    }

    Ok(produced)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{SiteId, CelestialBodyId, BodyType, Temperature, Atmosphere, StarSystemId};

    fn create_test_site() -> Site {
        let body_id = CelestialBodyId::new();
        Site::new_settlement(body_id, "Test Site".to_string(), 0, 100)
    }

    fn create_test_body() -> CelestialBody {
        CelestialBody::new(
            StarSystemId::new(),
            "Test Body".to_string(),
            BodyType::TerrestrialPlanet,
            Atmosphere::None,
            Temperature::Temperate,
            1.0,
            6371.0,
            1.0,
        )
    }

    fn create_test_content_with_recipe(recipe_yaml: &str) -> ContentLoader {
        let mut content = ContentLoader::new();
        content.load_recipes(recipe_yaml).expect("Failed to load recipe");
        content
    }

    #[test]
    fn test_idle_building_without_recipe() {
        let mut building = BuildingInstance::new_operational(
            SiteId::new(),
            "smelter".to_string(),
            0,
        );
        let mut site = create_test_site();
        site.add_building(building.clone());
        let mut body = create_test_body();
        let content = ContentLoader::new();

        let result = process_building_production(building.id, &mut site, &mut body, &content);

        assert!(result.is_ok());
        match result.unwrap() {
            ProductionResult::Idle { building_id } => {
                assert_eq!(building_id, building.id);
            }
            _ => panic!("Expected Idle result"),
        }
    }

    #[test]
    fn test_paused_building_is_idle() {
        let mut building = BuildingInstance::new_operational(
            SiteId::new(),
            "smelter".to_string(),
            0,
        );
        building.set_recipe("smelt_iron".to_string());
        building.pause();

        let mut site = create_test_site();
        site.add_building(building.clone());
        let mut body = create_test_body();
        let content = ContentLoader::new();

        let result = process_building_production(building.id, &mut site, &mut body, &content);

        assert!(result.is_ok());
        match result.unwrap() {
            ProductionResult::Idle { .. } => {}
            _ => panic!("Expected Idle result for paused building"),
        }
    }

    #[test]
    fn test_insufficient_inputs_halts_production() {
        let mut building = BuildingInstance::new_operational(
            SiteId::new(),
            "smelter".to_string(),
            0,
        );
        building.set_recipe("smelt_copper".to_string());

        let mut site = create_test_site();
        site.add_building(building.clone());
        // Site starts with 50 iron_ore but no copper_ore
        
        let mut body = create_test_body();
        
        let recipe_yaml = r#"
recipes:
  - id: smelt_copper
    name: "Smelt Copper"
    building_types: [smelter]
    inputs:
      copper_ore: 15
      power: 20
    outputs:
      copper_ingots: 10
    processing_time_ticks: 2
"#;
        let content = create_test_content_with_recipe(recipe_yaml);

        let result = process_building_production(building.id, &mut site, &mut body, &content);

        assert!(result.is_ok());
        match result.unwrap() {
            ProductionResult::Halted { reason, .. } => {
                match reason {
                    ProductionError::InsufficientResource { resource, .. } => {
                        assert_eq!(resource, "copper_ore");
                    }
                    _ => panic!("Expected InsufficientResource error"),
                }
            }
            _ => panic!("Expected Halted result"),
        }
    }

    #[test]
    fn test_production_consumes_inputs_and_progresses() {
        let mut building = BuildingInstance::new_operational(
            SiteId::new(),
            "smelter".to_string(),
            0,
        );
        building.set_recipe("smelt_iron".to_string());

        let mut site = create_test_site();
        site.add_building(building.clone());
        // Add iron ore to stockpile
        site.set_resource_by_id("iron_ore", 100.0).unwrap();
        
        let mut body = create_test_body();
        
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
    processing_time_ticks: 2
"#;
        let content = create_test_content_with_recipe(recipe_yaml);

        // First tick: consumes inputs, starts progress
        let result = process_building_production(building.id, &mut site, &mut body, &content);

        assert!(result.is_ok());
        match result.unwrap() {
            ProductionResult::InProgress { progress_ticks, processing_time_ticks, .. } => {
                assert_eq!(progress_ticks, 1);
                assert_eq!(processing_time_ticks, 2);
            }
            _ => panic!("Expected InProgress result"),
        }

        // Verify inputs were consumed
        let iron_ore = site.get_resource_by_id("iron_ore").unwrap();
        assert_eq!(iron_ore, 85.0); // 100 - 15
    }

    #[test]
    fn test_production_completes_and_produces_outputs() {
        let mut building = BuildingInstance::new_operational(
            SiteId::new(),
            "smelter".to_string(),
            0,
        );
        building.set_recipe("smelt_iron".to_string());

        let mut site = create_test_site();
        site.add_building(building.clone());
        // Site starts with 50 iron_ore, set to 100
        site.set_resource_by_id("iron_ore", 100.0).unwrap();
        // Clear starting steel (it maps to iron_ingots)
        site.set_resource_by_id("iron_ingots", 0.0).unwrap();
        
        let mut body = create_test_body();
        
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
    processing_time_ticks: 2
"#;
        let content = create_test_content_with_recipe(recipe_yaml);

        // Tick 1: Start recipe, consume inputs, progress to 1
        let result1 = process_building_production(building.id, &mut site, &mut body, &content);
        assert!(matches!(result1.unwrap(), ProductionResult::InProgress { .. }));

        // Tick 2: Complete recipe, produce outputs, reset progress
        let result2 = process_building_production(building.id, &mut site, &mut body, &content);
        
        assert!(result2.is_ok());
        match result2.unwrap() {
            ProductionResult::Completed { outputs_produced, .. } => {
                assert_eq!(outputs_produced.get("iron_ingots"), Some(&10.0));
            }
            _ => panic!("Expected Completed result"),
        }

        // Verify outputs were produced (iron_ingots maps to Steel)
        let iron_ingots = site.get_resource_by_id("iron_ingots").unwrap();
        assert_eq!(iron_ingots, 10.0);

        // Verify building is ready for next cycle
        assert_eq!(building.recipe_progress_ticks, 0);
    }

    #[test]
    fn test_multiple_production_cycles() {
        let mut building = BuildingInstance::new_operational(
            SiteId::new(),
            "smelter".to_string(),
            0,
        );
        building.set_recipe("smelt_iron".to_string());

        let mut site = create_test_site();
        site.add_building(building.clone());
        site.set_resource_by_id("iron_ore", 100.0).unwrap();
        // Clear starting steel
        site.set_resource_by_id("iron_ingots", 0.0).unwrap();
        
        let mut body = create_test_body();
        
        let recipe_yaml = r#"
recipes:
  - id: smelt_iron
    name: "Smelt Iron"
    building_types: [smelter]
    inputs:
      iron_ore: 15
    outputs:
      iron_ingots: 10
    processing_time_ticks: 2
"#;
        let content = create_test_content_with_recipe(recipe_yaml);

        // Run 3 complete cycles (6 ticks)
        for cycle in 0..3 {
            // Tick 1 of cycle: start
            let r1 = process_building_production(building.id, &mut site, &mut body, &content);
            assert!(matches!(r1.unwrap(), ProductionResult::InProgress { .. }));

            // Tick 2 of cycle: complete
            let r2 = process_building_production(building.id, &mut site, &mut body, &content);
            assert!(matches!(r2.unwrap(), ProductionResult::Completed { .. }));

            // Check accumulated output
            let expected_output = (cycle + 1) * 10;
            let iron_ingots = site.get_resource_by_id("iron_ingots").unwrap();
            assert_eq!(iron_ingots, expected_output as f64);
        }

        // After 3 cycles: consumed 45 iron_ore (3 * 15)
        let iron_ore = site.get_resource_by_id("iron_ore").unwrap();
        assert_eq!(iron_ore, 55.0); // 100 - 45
    }

    #[test]
    fn test_virtual_resources_not_consumed() {
        let mut building = BuildingInstance::new_operational(
            SiteId::new(),
            "smelter".to_string(),
            0,
        );
        building.set_recipe("smelt_iron".to_string());

        let mut site = create_test_site();
        site.add_building(building.clone());
        site.set_resource_by_id("iron_ore", 100.0).unwrap();
        
        let mut body = create_test_body();
        
        let recipe_yaml = r#"
recipes:
  - id: smelt_iron
    name: "Smelt Iron"
    building_types: [smelter]
    inputs:
      iron_ore: 15
      power: 100   # Virtual resource
      labor: 5     # Virtual resource
    outputs:
      iron_ingots: 10
    processing_time_ticks: 1
"#;
        let content = create_test_content_with_recipe(recipe_yaml);

        // Process one complete cycle
        let result = process_building_production(building.id, &mut site, &mut body, &content);
        assert!(matches!(result.unwrap(), ProductionResult::Completed { .. }));

        // Virtual resources (power, labor) should still be 0 (not consumed)
        assert_eq!(site.get_resource_by_id("power").unwrap(), 0.0);
        assert_eq!(site.get_resource_by_id("labor").unwrap(), 0.0);
    }
}

// ── Property tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::domain::{SiteId, CelestialBodyId, BodyType, Temperature, Atmosphere, StarSystemId};
    use proptest::prelude::*;

    fn make_site_with_ore(ore: f64) -> Site {
        let body_id = CelestialBodyId::new();
        let mut site = Site::new_settlement(body_id, "Prop Test".to_string(), 0, 100);
        site.set_resource_by_id("iron_ore", ore).unwrap();
        site.set_resource_by_id("iron_ingots", 0.0).unwrap();
        site
    }

    fn make_body() -> CelestialBody {
        CelestialBody::new(
            StarSystemId::new(),
            "Body".to_string(),
            BodyType::TerrestrialPlanet,
            Atmosphere::None,
            Temperature::Temperate,
            1.0,
            6371.0,
            1.0,
        )
    }

    fn smelt_iron_content(processing_ticks: u64, input: f64, output: f64) -> ContentLoader {
        let yaml = format!(
            r#"
recipes:
  - id: smelt_iron
    name: "Smelt Iron"
    building_types: [smelter]
    inputs:
      iron_ore: {input}
    outputs:
      iron_ingots: {output}
    processing_time_ticks: {processing_ticks}
"#,
            input = input,
            output = output,
            processing_ticks = processing_ticks,
        );
        let mut content = ContentLoader::new();
        content.load_recipes(&yaml).expect("load recipes");
        content
    }

    proptest! {
        /// After a completed production cycle, the sum (ore_remaining + ingots_produced * conversion_ratio)
        /// equals the initial ore. Specifically: initial_ore = ore_consumed + ore_remaining.
        #[test]
        fn prop_inputs_fully_conserved(
            initial_ore in 15.0f64..1000.0,
        ) {
            let content = smelt_iron_content(1, 15.0, 10.0);
            let mut site = make_site_with_ore(initial_ore);
            let mut body = make_body();

            let mut building = BuildingInstance::new_operational(site.id, "smelter".to_string(), 0);
            building.set_recipe("smelt_iron".to_string());
            let building_id = building.id;
            site.buildings.insert(building_id, building);

            let before_ore = site.get_resource_by_id("iron_ore").unwrap();

            let result = process_building_production(building_id, &mut site, &mut body, &content);
            prop_assert!(result.is_ok());

            let after_ore = site.get_resource_by_id("iron_ore").unwrap();
            let after_ingots = site.get_resource_by_id("iron_ingots").unwrap();

            match result.unwrap() {
                ProductionResult::Completed { .. } => {
                    // Exactly 15 ore consumed and 10 ingots produced
                    prop_assert!((before_ore - after_ore - 15.0).abs() < 1e-9,
                        "Expected 15 ore consumed, got {}", before_ore - after_ore);
                    prop_assert!((after_ingots - 10.0).abs() < 1e-9,
                        "Expected 10 ingots produced, got {}", after_ingots);
                }
                ProductionResult::InProgress { .. } => {
                    // Progress tick — ore already consumed at cycle start
                    prop_assert!((before_ore - after_ore - 15.0).abs() < 1e-9,
                        "Inputs consumed on cycle start");
                }
                ProductionResult::Halted { .. } | ProductionResult::Idle { .. } => {
                    // No state change expected
                    prop_assert!((before_ore - after_ore).abs() < 1e-9,
                        "Halted/idle should not change stockpile");
                }
            }
        }

        /// After a completed cycle the outputs are exactly the recipe output amounts.
        #[test]
        fn prop_outputs_match_recipe(
            output_amount in 1.0f64..500.0,
        ) {
            let content = smelt_iron_content(1, 15.0, output_amount);
            let mut site = make_site_with_ore(1000.0); // enough ore, enough storage
            let mut body = make_body();

            let mut building = BuildingInstance::new_operational(site.id, "smelter".to_string(), 0);
            building.set_recipe("smelt_iron".to_string());
            let building_id = building.id;
            site.buildings.insert(building_id, building);

            let result = process_building_production(building_id, &mut site, &mut body, &content);
            prop_assert!(result.is_ok());

            if let ProductionResult::Completed { outputs_produced, .. } = result.unwrap() {
                let produced = outputs_produced.get("iron_ingots").copied().unwrap_or(0.0);
                prop_assert!((produced - output_amount).abs() < 1e-9,
                    "Produced {} but expected {}", produced, output_amount);
            }
        }

        /// With insufficient inputs the stockpile is never modified.
        #[test]
        fn prop_halted_does_not_modify_stockpile(
            initial_ore in 0i64..=14i64,   // always less than the 15 required (stored as i64)
        ) {
            let content = smelt_iron_content(1, 15.0, 10.0);
            let mut site = make_site_with_ore(initial_ore as f64);
            let mut body = make_body();

            let mut building = BuildingInstance::new_operational(site.id, "smelter".to_string(), 0);
            building.set_recipe("smelt_iron".to_string());
            let building_id = building.id;
            site.buildings.insert(building_id, building);

            let before_ore = site.get_resource_by_id("iron_ore").unwrap();
            let before_ingots = site.get_resource_by_id("iron_ingots").unwrap();

            let result = process_building_production(building_id, &mut site, &mut body, &content);
            prop_assert!(result.is_ok());
            match result.unwrap() {
                ProductionResult::Halted { .. } => {}
                other => prop_assert!(false, "Expected Halted, got {:?}", other),
            }

            let after_ore = site.get_resource_by_id("iron_ore").unwrap();
            let after_ingots = site.get_resource_by_id("iron_ingots").unwrap();
            prop_assert!((before_ore - after_ore).abs() < 1e-9, "Ore changed on halt");
            prop_assert!((before_ingots - after_ingots).abs() < 1e-9, "Ingots changed on halt");
        }
    }
}
