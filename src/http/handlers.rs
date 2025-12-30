use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};

use crate::domain::*;
use crate::events::{EventStore, GameEvent, EventType};
use crate::commands::{Command, FoundColony, ConstructBuilding, AdvanceTurn};
use crate::db::DbPool;

#[derive(Serialize)]
struct ResourcesView {
    credits: i64,
    energy: i64,
    iron_ore: i64,
    food: i64,
    water: i64,
}

impl From<&Resources> for ResourcesView {
    fn from(resources: &Resources) -> Self {
        Self {
            credits: resources.get(ResourceType::Credits),
            energy: resources.get(ResourceType::Energy),
            iron_ore: resources.get(ResourceType::IronOre),
            food: resources.get(ResourceType::Food),
            water: resources.get(ResourceType::Water),
        }
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
            population: colony.population,
            morale: colony.morale,
            pollution_level: colony.pollution_level,
        }
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
            Colony {
                id: ColonyId(id),
                planet_id: PlanetId(planet_id),
                name,
                founded_at: chrono::DateTime::parse_from_rfc3339(&founded_at)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                resources: Resources::starting_resources(), // TODO: Load from db
                population,
                morale,
                pollution_level,
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
                    new_colony.population,
                    new_colony.morale,
                    new_colony.pollution_level,
                ],
            ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

            new_colony
        }
    };

    // Get buildings for this colony
    let mut stmt = conn.prepare(
        "SELECT building_id, building_type, state FROM buildings WHERE colony_id = ?1"
    ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let buildings: Vec<(u64, String, String)> = stmt.query_map([*colony_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })
    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
    .filter_map(|r| r.ok())
    .collect();

    let colony_view = ColonyView::from(&colony);
    let resources_view = ResourcesView::from(&colony.resources);

    let mut context = tera::Context::new();
    context.insert("colony", &colony_view);
    context.insert("resources", &resources_view);
    context.insert("buildings_count", &buildings.len());

    let html = tmpl.render("colony.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

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

    let command = ConstructBuilding {
        building_id: BuildingId(building_id),
        colony_id: ColonyId(*colony_id),
        building_type: building_type.clone(),
        available_resources: Resources::starting_resources(), // TODO: Get actual resources
    };

    let events = command.execute()
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // Save events
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
        if let EventType::BuildingConstructionStarted { building_id, colony_id, building_type } = event {
            conn.execute(
                "INSERT INTO buildings (building_id, colony_id, building_type, state) VALUES (?1, ?2, ?3, 'UnderConstruction')",
                rusqlite::params![
                    building_id.0,
                    colony_id.0,
                    serde_json::to_string(&building_type).unwrap(),
                ],
            ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        }
    }

    Ok(HttpResponse::Ok()
        .insert_header(("HX-Trigger", "buildingAdded"))
        .body("Building construction started"))
}

pub async fn advance_turn(
    _colony_id: web::Path<u64>,
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

    let command = AdvanceTurn { current_turn };

    let events = command.execute()
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // Save events
    for event in events {
        let game_event = GameEvent::new(
            event_store.get_next_event_id()
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
            current_turn,
            event.clone(),
        );

        event_store.save_event(&game_event)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        // Update game state
        if let EventType::TurnAdvanced { turn_number } = event {
            conn.execute(
                "UPDATE game_state SET current_turn = ?1 WHERE id = 1",
                [turn_number],
            ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        }
    }

    Ok(HttpResponse::Ok()
        .insert_header(("HX-Refresh", "true"))
        .body(format!("Turn advanced to {}", current_turn + 1)))
}
