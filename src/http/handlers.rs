use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use rusqlite::Connection;

use crate::domain::*;
use crate::events::{EventStore, GameEvent, EventType};
use crate::commands::{Command, FoundColony, ConstructBuilding, UpgradeBuilding, RepairBuilding, AdvanceTurn};
use crate::db::DbPool;

// Helper functions for database operations
fn load_resources(conn: &Connection, colony_id: u64) -> anyhow::Result<Resources> {
    let mut resources = Resources::new();

    let mut stmt = conn.prepare(
        "SELECT resource_type, quantity FROM resource_stockpiles WHERE colony_id = ?1"
    )?;

    let rows = stmt.query_map([colony_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;

    for row_result in rows {
        let (resource_type_str, quantity) = row_result?;

        // Parse resource type string back to enum
        // The string format is the Debug representation (e.g., "IronOre", "AdvancedComponents")
        let resource_type = match resource_type_str.as_str() {
            "Credits" => ResourceType::Credits,
            "Energy" => ResourceType::Energy,
            "IronOre" => ResourceType::IronOre,
            "CopperOre" => ResourceType::CopperOre,
            "RareMetals" => ResourceType::RareMetals,
            "Water" => ResourceType::Water,
            "Timber" => ResourceType::Timber,
            "Food" => ResourceType::Food,
            "Coal" => ResourceType::Coal,
            "Uranium" => ResourceType::Uranium,
            "Silicon" => ResourceType::Silicon,
            "Titanium" => ResourceType::Titanium,
            "Platinum" => ResourceType::Platinum,
            "Aluminum" => ResourceType::Aluminum,
            "CrystalOre" => ResourceType::CrystalOre,
            "Gas" => ResourceType::Gas,
            "Oil" => ResourceType::Oil,
            "Steel" => ResourceType::Steel,
            "Electronics" => ResourceType::Electronics,
            "Machinery" => ResourceType::Machinery,
            "Concrete" => ResourceType::Concrete,
            "Plastics" => ResourceType::Plastics,
            "Fuel" => ResourceType::Fuel,
            "Chemicals" => ResourceType::Chemicals,
            "AdvancedComponents" => ResourceType::AdvancedComponents,
            "Medicine" => ResourceType::Medicine,
            "Research" => ResourceType::Research,
            "ConsumerGoods" => ResourceType::ConsumerGoods,
            "Luxuries" => ResourceType::Luxuries,
            _ => continue, // Skip unknown resource types for forward compatibility
        };
        resources.set(resource_type, quantity);
    }

    // If no resources in DB, initialize with starting resources
    if resources.get(ResourceType::Credits) == 0 {
        resources = Resources::starting_resources();
        save_resources(conn, colony_id, &resources)?;
    }

    Ok(resources)
}

fn save_resources(conn: &Connection, colony_id: u64, resources: &Resources) -> anyhow::Result<()> {
    for (resource_type, quantity) in resources.iter() {
        let resource_type_str = format!("{:?}", resource_type);
        conn.execute(
            "INSERT OR REPLACE INTO resource_stockpiles (colony_id, resource_type, quantity)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![colony_id, resource_type_str, quantity],
        )?;
    }
    Ok(())
}

#[derive(Serialize)]
struct ResourceItem {
    name: String,
    quantity: i64,
    resource_type: String,
}

#[derive(Serialize)]
struct ResourceCategory {
    name: String,
    resources: Vec<ResourceItem>,
}

#[derive(Serialize)]
struct ResourcesView {
    categories: Vec<ResourceCategory>,
}

impl From<&Resources> for ResourcesView {
    fn from(resources: &Resources) -> Self {
        use std::collections::HashMap;

        // Group resources by category
        let mut categories_map: HashMap<String, Vec<ResourceItem>> = HashMap::new();

        // Define all resource types to ensure consistent ordering
        let all_resource_types = vec![
            ResourceType::Credits,
            ResourceType::Energy,
            ResourceType::IronOre,
            ResourceType::CopperOre,
            ResourceType::RareMetals,
            ResourceType::Water,
            ResourceType::Timber,
            ResourceType::Food,
            ResourceType::Coal,
            ResourceType::Uranium,
            ResourceType::Silicon,
            ResourceType::Titanium,
            ResourceType::Platinum,
            ResourceType::Aluminum,
            ResourceType::CrystalOre,
            ResourceType::Gas,
            ResourceType::Oil,
            ResourceType::Steel,
            ResourceType::Electronics,
            ResourceType::Machinery,
            ResourceType::Concrete,
            ResourceType::Plastics,
            ResourceType::Fuel,
            ResourceType::Chemicals,
            ResourceType::AdvancedComponents,
            ResourceType::Medicine,
            ResourceType::Research,
            ResourceType::ConsumerGoods,
            ResourceType::Luxuries,
        ];

        for resource_type in all_resource_types {
            let quantity = resources.get(resource_type);
            let category = resource_type.category().to_string();
            let item = ResourceItem {
                name: resource_type.name().to_string(),
                quantity,
                resource_type: format!("{:?}", resource_type),
            };

            categories_map
                .entry(category)
                .or_insert_with(Vec::new)
                .push(item);
        }

        // Convert HashMap to sorted Vec of categories
        let category_order = vec![
            "Currency",
            "Energy",
            "Raw Materials - Basic",
            "Raw Materials - Advanced",
            "Processed Goods - Basic",
            "Processed Goods - Advanced",
            "Specialized",
        ];

        let categories = category_order
            .iter()
            .filter_map(|cat_name| {
                categories_map.remove(*cat_name).map(|resources| ResourceCategory {
                    name: cat_name.to_string(),
                    resources,
                })
            })
            .collect();

        Self { categories }
    }
}

#[derive(Serialize)]
struct ColonyView {
    id: u64,
    planet_id: u64,
    name: String,
    founded_at: String,
    population: u64,
    morale: f32,
    pollution_level: f32,
}

impl From<&Colony> for ColonyView {
    fn from(colony: &Colony) -> Self {
        Self {
            id: colony.id.0,
            planet_id: colony.planet_id.0,
            name: colony.name.clone(),
            founded_at: colony.founded_at.format("%Y-%m-%d %H:%M UTC").to_string(),
            population: colony.population.total,
            morale: colony.morale(),
            pollution_level: colony.pollution_level,
        }
    }
}

#[derive(Serialize)]
struct BuildingView {
    id: u64,
    type_name: String,
    name: String,
    state: String,
    description: String,
    workers_assigned: u32,
    worker_capacity: u32,
    efficiency: f32,
    power_consumption: u32,
    production_output: String,
    level: u8,
    can_upgrade: bool,
    is_damaged: bool,
}

impl BuildingView {
    fn from_building(building: &Building) -> Self {
        let (type_name, description) = match &building.building_type {
            BuildingType::Mine { resource_type, output_rate } => {
                (format!("Mine ({:?})", resource_type), format!("Produces {} {:?} per turn", output_rate, resource_type))
            }
            BuildingType::PowerPlant { output_mw, .. } => {
                ("Power Plant".to_string(), format!("Generates {} MW", output_mw))
            }
            BuildingType::Housing { capacity, .. } => {
                ("Housing".to_string(), format!("Capacity: {} residents", capacity))
            }
            BuildingType::Farm { output_rate } => {
                ("Farm".to_string(), format!("Produces {} Food per turn", output_rate))
            }
            BuildingType::Factory { produces, .. } => {
                ("Factory".to_string(), format!("Produces {:?}", produces))
            }
            BuildingType::Refinery { output_type, .. } => {
                ("Refinery".to_string(), format!("Produces {:?}", output_type))
            }
            BuildingType::ResearchFacility { .. } => {
                ("Research Facility".to_string(), "Generates research points".to_string())
            }
            BuildingType::MedicalFacility { .. } => {
                ("Medical Facility".to_string(), "Produces medicine, boosts morale".to_string())
            }
            BuildingType::CommercialZone { .. } => {
                ("Commercial Zone".to_string(), "Produces consumer goods and credits".to_string())
            }
            BuildingType::SolarPowerPlant { output_mw } => {
                ("Solar Power Plant".to_string(), format!("Generates {} MW", output_mw))
            }
            BuildingType::NuclearPowerPlant { output_mw, .. } => {
                ("Nuclear Power Plant".to_string(), format!("Generates {} MW", output_mw))
            }
            _ => ("Unknown".to_string(), "".to_string()),
        };

        let state_str = match building.state {
            BuildingState::Operational => "Operational".to_string(),
            BuildingState::UnderConstruction { progress } => {
                format!("Under Construction ({}%)", building.construction_progress_percentage() as u8)
            }
            BuildingState::Damaged { severity } => format!("Damaged ({}%)", severity),
            BuildingState::Shutdown => "Shutdown".to_string(),
        };

        // Get production output description
        let outputs = building.production_outputs();
        let production_output = if outputs.iter().count() > 0 {
            outputs.iter()
                .map(|(rt, amount)| format!("{} {:?}", amount, rt))
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            "None".to_string()
        };

        Self {
            id: building.id.0,
            type_name: type_name.clone(),
            name: type_name,
            state: state_str,
            description,
            workers_assigned: building.workers_assigned,
            worker_capacity: building.worker_capacity(),
            efficiency: building.production_efficiency() * 100.0,
            power_consumption: building.power_consumption(),
            production_output,
            level: building.level,
            can_upgrade: building.can_upgrade(),
            is_damaged: matches!(building.state, BuildingState::Damaged { .. }),
        }
    }

    fn from_type(id: u64, building_type: &BuildingType, state: &str) -> Self {
        // Create a temporary building for the view
        let temp_building = Building {
            id: BuildingId(id),
            colony_id: ColonyId(0),
            building_type: building_type.clone(),
            state: if state.contains("Operational") {
                BuildingState::Operational
            } else if state.contains("Construction") {
                BuildingState::UnderConstruction { progress: 0 }
            } else {
                BuildingState::Shutdown
            },
            workers_assigned: 0,
            level: 1,
        };
        Self::from_building(&temp_building)
    }
}

pub async fn index(tmpl: web::Data<tera::Tera>) -> Result<HttpResponse> {
    let mut context = tera::Context::new();
    context.insert("title", "Outpost 3: Wormhole Empire");

    let html = tmpl.render("index.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

pub async fn view_colony(
    colony_id: web::Path<u64>,
    tmpl: web::Data<tera::Tera>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse> {
    let conn = pool.get()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Query colony from database
    let colony_result: std::result::Result<(u64, u64, String, String, u64, f32, f32), rusqlite::Error> = conn.query_row(
        "SELECT colony_id, planet_id, name, founded_at, population, morale, pollution_level FROM colonies WHERE colony_id = ?1",
        [*colony_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        },
    );

    let colony = match colony_result {
        Ok((id, planet_id, name, founded_at, population, morale, pollution_level)) => {
            let resources = load_resources(&conn, id)
                .unwrap_or_else(|_| Resources::starting_resources());

            {
                let mut pop = Population::new(population);
                pop.morale = morale;
                Colony {
                    id: ColonyId(id),
                    planet_id: PlanetId(planet_id),
                    name,
                    founded_at: chrono::DateTime::parse_from_rfc3339(&founded_at)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    resources,
                    population: pop,
                    power_grid: PowerGrid::new(),
                    pollution_level,
                }
            }
        }
        Err(_) => {
            // If colony doesn't exist, create a default one
            let new_colony = Colony::new(
                ColonyId(*colony_id),
                PlanetId(1),
                "New Hope".to_string(),
            );

            // Insert into database
            conn.execute(
                "INSERT OR IGNORE INTO colonies (colony_id, planet_id, name, founded_at, population, morale, pollution_level) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    new_colony.id.0,
                    new_colony.planet_id.0,
                    &new_colony.name,
                    new_colony.founded_at.to_rfc3339(),
                    new_colony.population.total,
                    new_colony.morale(),
                    new_colony.pollution_level,
                ],
            ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

            new_colony
        }
    };

    // Get buildings for this colony
    let mut stmt = conn.prepare(
        "SELECT building_id, building_type, state, workers_assigned, level FROM buildings WHERE colony_id = ?1"
    ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let building_data: Vec<(u64, String, String, u32, u8)> = stmt.query_map([*colony_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4).unwrap_or(1)))
    })
    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
    .filter_map(|r| r.ok())
    .collect();

    // Convert to BuildingView by creating actual Building instances
    let buildings: Vec<BuildingView> = building_data.iter()
        .filter_map(|(id, type_str, state_str, workers, level)| {
            let building_type = serde_json::from_str::<BuildingType>(type_str).ok()?;

            // Parse state from JSON, fallback to string parsing for backward compatibility
            let state = serde_json::from_str::<BuildingState>(state_str)
                .unwrap_or_else(|_| {
                    // Fallback for old string-based states
                    if state_str.contains("Operational") {
                        BuildingState::Operational
                    } else if state_str.contains("Construction") {
                        BuildingState::UnderConstruction { progress: 0 }
                    } else if state_str.contains("Damaged") {
                        BuildingState::Damaged { severity: 50 }
                    } else {
                        BuildingState::Shutdown
                    }
                });

            // Create a full Building instance
            let building = Building {
                id: BuildingId(*id),
                colony_id: ColonyId(*colony_id),
                building_type,
                state,
                workers_assigned: *workers,
                level: *level,
            };

            Some(BuildingView::from_building(&building))
        })
        .collect();

    let colony_view = ColonyView::from(&colony);
    let resources_view = ResourcesView::from(&colony.resources);

    // Calculate actual employment from buildings
    let total_employed: u32 = buildings.iter().map(|b| b.workers_assigned).sum();
    let mut colony_mut = colony.clone();
    colony_mut.population.employed = total_employed as u64;
    colony_mut.population.unemployed = colony_mut.population.total.saturating_sub(total_employed as u64);

    // Prepare power grid data
    let power_grid_data = serde_json::json!({
        "generation": colony_mut.power_grid.total_generation,
        "consumption": colony_mut.power_grid.total_consumption,
        "net_power": colony_mut.power_grid.net_power(),
        "utilization": colony_mut.power_grid.utilization_percentage(),
        "has_brownout": colony_mut.power_grid.has_brownout(),
        "deficit": colony_mut.power_grid.power_deficit(),
        "efficiency": colony_mut.power_grid.efficiency_rating(),
    });

    // Prepare population data
    let population_data = serde_json::json!({
        "total": colony_mut.population.total,
        "growth_rate": colony_mut.population.growth_rate * 100.0,
        "morale": colony_mut.population.morale,
        "employed": colony_mut.population.employed,
        "unemployed": colony_mut.population.unemployed,
        "unemployment_rate": colony_mut.population.unemployment_rate(),
        "housing_capacity": colony_mut.population.housing_capacity,
        "is_overcrowded": colony_mut.population.is_overcrowded(),
        "occupancy_rate": if colony_mut.population.housing_capacity > 0 {
            (colony_mut.population.total as f32 / colony_mut.population.housing_capacity as f32) * 100.0
        } else {
            0.0
        },
        "food_needed": colony_mut.population.food_needed_per_turn(),
    });

    // Calculate resource flows from operational buildings
    // Re-query buildings to get full Building objects for production calculations
    let mut resource_flows = Vec::new();
    let mut net_production: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for (id, type_str, state_str, workers, level) in &building_data {
        if let (Ok(building_type), Ok(state)) = (
            serde_json::from_str::<BuildingType>(type_str),
            serde_json::from_str::<BuildingState>(state_str)
        ) {
            // Skip if not operational
            if !matches!(state, BuildingState::Operational) {
                continue;
            }

            let temp_building = Building {
                id: BuildingId(*id),
                colony_id: ColonyId(*colony_id),
                building_type: building_type.clone(),
                state,
                workers_assigned: *workers,
                level: *level,
            };

            let inputs = temp_building.production_inputs();
            let outputs = temp_building.production_outputs();

            // Only include buildings with actual production
            if inputs.iter().count() > 0 || outputs.iter().count() > 0 {
                let mut input_list = Vec::new();
                for (resource_type, amount) in inputs.iter() {
                    input_list.push(serde_json::json!({
                        "name": format!("{:?}", resource_type),
                        "amount": amount,
                    }));
                    *net_production.entry(format!("{:?}", resource_type)).or_insert(0) -= amount;
                }

                let mut output_list = Vec::new();
                for (resource_type, amount) in outputs.iter() {
                    output_list.push(serde_json::json!({
                        "name": format!("{:?}", resource_type),
                        "amount": amount,
                    }));
                    *net_production.entry(format!("{:?}", resource_type)).or_insert(0) += amount;
                }

                // Get building name from type
                let building_name = match &building_type {
                    BuildingType::Mine { resource_type, .. } => format!("Mine ({:?})", resource_type),
                    BuildingType::PowerPlant { .. } => "Power Plant".to_string(),
                    BuildingType::Housing { .. } => "Housing".to_string(),
                    BuildingType::Farm { .. } => "Farm".to_string(),
                    BuildingType::Factory { produces, .. } => format!("Factory ({:?})", produces),
                    BuildingType::Refinery { output_type, .. } => format!("Refinery ({:?})", output_type),
                    BuildingType::ResearchFacility { .. } => "Research Facility".to_string(),
                    BuildingType::MedicalFacility { .. } => "Medical Facility".to_string(),
                    BuildingType::CommercialZone { .. } => "Commercial Zone".to_string(),
                    BuildingType::SolarPowerPlant { .. } => "Solar Power Plant".to_string(),
                    BuildingType::NuclearPowerPlant { .. } => "Nuclear Power Plant".to_string(),
                    _ => "Building".to_string(),
                };

                resource_flows.push(serde_json::json!({
                    "building_name": building_name,
                    "efficiency": (temp_building.production_efficiency() * 100.0) as u32,
                    "inputs": input_list,
                    "outputs": output_list,
                }));
            }
        }
    }

    // Convert net production to list format
    let mut net_production_list: Vec<serde_json::Value> = net_production.iter()
        .map(|(name, net)| serde_json::json!({
            "name": name,
            "net": net,
        }))
        .collect();

    // Sort by resource name
    net_production_list.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    let mut context = tera::Context::new();
    context.insert("colony", &colony_view);
    context.insert("resources", &resources_view);
    context.insert("buildings", &buildings);
    context.insert("buildings_count", &buildings.len());
    context.insert("power_grid", &power_grid_data);
    context.insert("population", &population_data);
    context.insert("available_workers", &colony_mut.population.unemployed);
    context.insert("colony_id", &colony_mut.id.0);
    context.insert("resource_flows", &resource_flows);
    context.insert("net_production", &net_production_list);

    let html = tmpl.render("colony.html", &context)
        .map_err(|e| {
            eprintln!("Template rendering error: {:?}", e);
            eprintln!("Error details: {}", e);
            if let Some(source) = std::error::Error::source(&e) {
                eprintln!("Caused by: {:?}", source);
            }
            actix_web::error::ErrorInternalServerError(e)
        })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Deserialize)]
pub struct CreateColonyForm {
    name: String,
    planet_id: u64,
}

pub async fn create_colony(
    form: web::Form<CreateColonyForm>,
    pool: web::Data<DbPool>,
    event_store: web::Data<EventStore>,
) -> Result<HttpResponse> {
    let conn = pool.get()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Get next colony ID
    let colony_id: u64 = conn.query_row(
        "SELECT COALESCE(MAX(colony_id), 0) + 1 FROM colonies",
        [],
        |row| row.get(0)
    ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let command = FoundColony {
        colony_id: ColonyId(colony_id),
        planet_id: PlanetId(form.planet_id),
        name: form.name.clone(),
    };

    let events = command.execute()
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // Save events and update database
    for event in events {
        let game_event = GameEvent::new(
            event_store.get_next_event_id()
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
            1,
            event.clone(),
        );

        event_store.save_event(&game_event)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        // Apply event to database
        if let EventType::ColonyFounded { colony_id, planet_id, name, .. } = event {
            conn.execute(
                "INSERT INTO colonies (colony_id, planet_id, name, founded_at, population, morale, pollution_level) VALUES (?1, ?2, ?3, ?4, 100, 75.0, 0.0)",
                rusqlite::params![colony_id.0, planet_id.0, name, chrono::Utc::now().to_rfc3339()],
            ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        }
    }

    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", format!("/colony/{}", colony_id)))
        .finish())
}

#[derive(Deserialize)]
pub struct ConstructBuildingForm {
    building_type: String,
}

pub async fn construct_building(
    colony_id: web::Path<u64>,
    form: web::Form<ConstructBuildingForm>,
    pool: web::Data<DbPool>,
    event_store: web::Data<EventStore>,
) -> Result<HttpResponse> {
    let conn = pool.get()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Get next building ID
    let building_id: u64 = conn.query_row(
        "SELECT COALESCE(MAX(building_id), 0) + 1 FROM buildings",
        [],
        |row| row.get(0)
    ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Parse building type
    let building_type = match form.building_type.as_str() {
        "mine" => BuildingType::Mine {
            resource_type: ResourceType::IronOre,
            output_rate: 10,
        },
        "power_plant" => BuildingType::PowerPlant {
            output_mw: 50,
            fuel_type: None,
        },
        "housing" => BuildingType::Housing {
            capacity: 100,
            comfort_level: 5,
        },
        _ => BuildingType::Farm { output_rate: 20 },
    };

    // Load actual resources
    let mut resources = load_resources(&conn, *colony_id)
        .unwrap_or_else(|_| Resources::starting_resources());

    let command = ConstructBuilding {
        building_id: BuildingId(building_id),
        colony_id: ColonyId(*colony_id),
        building_type: building_type.clone(),
        available_resources: resources.clone(),
    };

    let events = command.execute()
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // Save events and apply to state
    for event in events {
        let game_event = GameEvent::new(
            event_store.get_next_event_id()
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
            1,
            event.clone(),
        );

        event_store.save_event(&game_event)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        // Apply event to database
        match event {
            EventType::BuildingConstructionStarted { building_id, colony_id, building_type } => {
                let initial_state = BuildingState::UnderConstruction { progress: 0 };
                conn.execute(
                    "INSERT INTO buildings (building_id, colony_id, building_type, state, workers_assigned, level) VALUES (?1, ?2, ?3, ?4, 0, 1)",
                    rusqlite::params![
                        building_id.0,
                        colony_id.0,
                        serde_json::to_string(&building_type).unwrap(),
                        serde_json::to_string(&initial_state).unwrap(),
                    ],
                ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            }
            EventType::ResourcesConsumed { colony_id, resource_type, amount } => {
                let current = resources.get(resource_type);
                resources.set(resource_type, current - amount);
            }
            _ => {}
        }
    }

    // Save updated resources
    save_resources(&conn, *colony_id, &resources)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(HttpResponse::Ok()
        .insert_header(("HX-Trigger", "buildingAdded"))
        .body("Building construction started"))
}

pub async fn upgrade_building(
    path: web::Path<(u64, u64)>,
    pool: web::Data<DbPool>,
    event_store: web::Data<EventStore>,
) -> Result<HttpResponse> {
    let (colony_id, building_id) = path.into_inner();
    let conn = pool.get()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Load the building
    let (type_str, state_str, workers, current_level): (String, String, u32, u8) = conn.query_row(
        "SELECT building_type, state, workers_assigned, level FROM buildings WHERE building_id = ?1 AND colony_id = ?2",
        rusqlite::params![building_id, colony_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    ).map_err(|e| actix_web::error::ErrorNotFound(format!("Building not found: {}", e)))?;

    let building_type: BuildingType = serde_json::from_str(&type_str)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    let state: BuildingState = serde_json::from_str(&state_str)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let building = Building {
        id: BuildingId(building_id),
        colony_id: ColonyId(colony_id),
        building_type,
        state,
        workers_assigned: workers,
        level: current_level,
    };

    // Check if upgrade is possible
    if !building.can_upgrade() {
        return Err(actix_web::error::ErrorBadRequest("Building is already at max level"));
    }

    // Load colony resources
    let mut resources = load_resources(&conn, colony_id)
        .unwrap_or_else(|_| Resources::starting_resources());

    let upgrade_cost = building.upgrade_cost();

    // Create and execute command
    let command = UpgradeBuilding {
        building_id: BuildingId(building_id),
        colony_id: ColonyId(colony_id),
        current_level,
        available_resources: resources.clone(),
        upgrade_cost: upgrade_cost.clone(),
    };

    let events = command.execute()
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // Save events and apply to state
    for event in events {
        let game_event = GameEvent::new(
            event_store.get_next_event_id()
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
            1,
            event.clone(),
        );

        event_store.save_event(&game_event)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        // Apply event to database
        match event {
            EventType::BuildingUpgraded { building_id, new_level, .. } => {
                conn.execute(
                    "UPDATE buildings SET level = ?1 WHERE building_id = ?2",
                    rusqlite::params![new_level, building_id.0],
                ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            }
            EventType::ResourcesConsumed { resource_type, amount, .. } => {
                let current = resources.get(resource_type);
                resources.set(resource_type, current - amount);
            }
            _ => {}
        }
    }

    // Save updated resources
    save_resources(&conn, colony_id, &resources)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(HttpResponse::Ok()
        .insert_header(("HX-Trigger", "buildingUpgraded"))
        .body(format!("Building upgraded to level {}", current_level + 1)))
}

pub async fn repair_building(
    path: web::Path<(u64, u64)>,
    pool: web::Data<DbPool>,
    event_store: web::Data<EventStore>,
) -> Result<HttpResponse> {
    let (colony_id, building_id) = path.into_inner();
    let conn = pool.get()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Load the building
    let (type_str, state_str, workers, level): (String, String, u32, u8) = conn.query_row(
        "SELECT building_type, state, workers_assigned, level FROM buildings WHERE building_id = ?1 AND colony_id = ?2",
        rusqlite::params![building_id, colony_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    ).map_err(|e| actix_web::error::ErrorNotFound(format!("Building not found: {}", e)))?;

    let building_type: BuildingType = serde_json::from_str(&type_str)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    let state: BuildingState = serde_json::from_str(&state_str)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Check if building is damaged
    if !matches!(state, BuildingState::Damaged { .. }) {
        return Err(actix_web::error::ErrorBadRequest("Building is not damaged"));
    }

    // Load colony resources
    let mut resources = load_resources(&conn, colony_id)
        .unwrap_or_else(|_| Resources::starting_resources());

    // Create and execute command
    let command = RepairBuilding {
        building_id: BuildingId(building_id),
        colony_id: ColonyId(colony_id),
        current_state: state,
        available_resources: resources.clone(),
    };

    let events = command.execute()
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // Save events and apply to state
    for event in events {
        let game_event = GameEvent::new(
            event_store.get_next_event_id()
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
            1,
            event.clone(),
        );

        event_store.save_event(&game_event)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        // Apply event to database
        match event {
            EventType::BuildingRepaired { building_id, .. } => {
                let operational_state = BuildingState::Operational;
                conn.execute(
                    "UPDATE buildings SET state = ?1 WHERE building_id = ?2",
                    rusqlite::params![
                        serde_json::to_string(&operational_state).unwrap(),
                        building_id.0
                    ],
                ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            }
            EventType::ResourcesConsumed { resource_type, amount, .. } => {
                let current = resources.get(resource_type);
                resources.set(resource_type, current - amount);
            }
            _ => {}
        }
    }

    // Save updated resources
    save_resources(&conn, colony_id, &resources)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(HttpResponse::Ok()
        .insert_header(("HX-Trigger", "buildingRepaired"))
        .body("Building repaired successfully"))
}

pub async fn advance_turn(
    colony_id: web::Path<u64>,
    event_store: web::Data<EventStore>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse> {
    let conn = pool.get()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Get current turn
    let current_turn: u64 = conn.query_row(
        "SELECT current_turn FROM game_state WHERE id = 1",
        [],
        |row| row.get(0)
    ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Load buildings
    let mut buildings = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT building_type FROM buildings WHERE colony_id = ?1 AND state = 'Operational'"
    ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let building_rows = stmt.query_map([*colony_id], |row| {
        row.get::<_, String>(0)
    }).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    for row_result in building_rows {
        let building_type_str = row_result
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        if let Ok(building_type) = serde_json::from_str::<BuildingType>(&building_type_str) {
            buildings.push((ColonyId(*colony_id), building_type));
        }
    }

    // Load current resources for this colony
    let mut resources = load_resources(&conn, *colony_id)
        .unwrap_or_else(|_| Resources::starting_resources());

    // Also load buildings under construction to advance them
    let mut under_construction_stmt = conn.prepare(
        "SELECT building_id, building_type, state FROM buildings WHERE colony_id = ?1"
    ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let construction_rows: Vec<(u64, String, String)> = under_construction_stmt.query_map([*colony_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })
    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
    .filter_map(|r| r.ok())
    .collect();

    // Advance construction for buildings under construction
    for (building_id, type_str, state_str) in construction_rows {
        if let Ok(building_type) = serde_json::from_str::<BuildingType>(&type_str) {
            if let Ok(state) = serde_json::from_str::<BuildingState>(&state_str) {
                if let BuildingState::UnderConstruction { progress } = state {
                    let mut temp_building = Building {
                        id: BuildingId(building_id),
                        colony_id: ColonyId(*colony_id),
                        building_type,
                        state,
                        workers_assigned: 0,
                        level: 1,
                    };

                    let completed = temp_building.advance_construction();
                    let new_state = temp_building.state;

                    // Update state in database
                    conn.execute(
                        "UPDATE buildings SET state = ?1 WHERE building_id = ?2",
                        rusqlite::params![
                            serde_json::to_string(&new_state).unwrap(),
                            building_id,
                        ],
                    ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

                    if completed {
                        eprintln!("Building {} completed construction", building_id);
                    }
                }
            }
        }
    }

    let command = AdvanceTurn {
        current_turn,
        buildings,
    };

    let events = command.execute()
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // Save events and apply to state
    for event in events {
        let game_event = GameEvent::new(
            event_store.get_next_event_id()
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
            current_turn,
            event.clone(),
        );

        event_store.save_event(&game_event)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        // Apply events
        match event {
            EventType::TurnAdvanced { turn_number } => {
                conn.execute(
                    "UPDATE game_state SET current_turn = ?1 WHERE id = 1",
                    [turn_number],
                ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            }
            EventType::ResourcesProduced { resource_type, amount, .. } => {
                let current = resources.get(resource_type);
                resources.set(resource_type, current + amount);
            }
            _ => {}
        }
    }

    // Save updated resources
    save_resources(&conn, *colony_id, &resources)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(HttpResponse::Ok()
        .insert_header(("HX-Refresh", "true"))
        .body(format!("Turn advanced to {}", current_turn + 1)))
}
