use actix_web::{web, HttpResponse, Result};
use crate::db::DbPool;
use crate::events::{EventStore, GameEvent, EventType};
use crate::commands::{Command, AllocateLabor, DeallocateLabor, ChangeBuildingState, RepairBuilding};
use crate::domain::{ColonyId, BuildingId, BuildingState, BuildingType, Resources};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use super::{load_resources, save_resources};

#[derive(Deserialize)]
pub struct LaborAllocationForm {
    pub workers: u32,
}

pub async fn allocate_labor(
    path: web::Path<(u64, u64)>,
    form: web::Form<LaborAllocationForm>,
    pool: web::Data<DbPool>,
    event_store: web::Data<EventStore>,
) -> Result<HttpResponse> {
    let (colony_id, building_id) = path.into_inner();
    let conn = pool.get()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // 1. Get current building state
    let (workers_assigned, level): (u32, u8) = conn.query_row(
        "SELECT workers_assigned, level FROM buildings WHERE building_id = ?1 AND colony_id = ?2",
        rusqlite::params![building_id, colony_id],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).map_err(|e| actix_web::error::ErrorNotFound(format!("Building not found: {}", e)))?;

    // 2. Get colony population state
    let (total_population, employed_population): (u32, u32) = conn.query_row(
        "SELECT 
            (SELECT population FROM colonies WHERE colony_id = ?1) as total,
            (SELECT COALESCE(SUM(workers_assigned), 0) FROM buildings WHERE colony_id = ?1) as employed",
        rusqlite::params![colony_id],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let available_unemployed = total_population.saturating_sub(employed_population);
    
    // Calculate capacity based on building type/level (simplified for now, ideally fetch from domain)
    // For now assuming 10 workers per level as a safe fallback if we can't get type easily
    // In a real implementation we'd fetch building_type and call .worker_capacity()
    let building_capacity = (level as u32) * 50; 

    // 3. Create Command
    let command = AllocateLabor {
        colony_id: ColonyId(colony_id),
        building_id: BuildingId(building_id),
        workers_to_allocate: form.workers,
        available_unemployed,
        building_capacity,
        current_workers: workers_assigned,
    };

    // 4. Execute
    let events = command.execute()
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // 5. Persist Events
    for event in events {
        let game_event = GameEvent::new(
            event_store.get_next_event_id()
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
            1,
            event.clone(),
        );
        event_store.save_event(&game_event)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        // 6. Update Read Model
        if let EventType::LaborAllocated { building_id, workers_allocated, .. } = event {
            conn.execute(
                "UPDATE buildings SET workers_assigned = workers_assigned + ?1 WHERE building_id = ?2",
                rusqlite::params![workers_allocated, building_id.0],
            ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        }
    }

    // Return HTMX partial or redirect
    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", format!("/colony/{}", colony_id)))
        .finish())
}

pub async fn deallocate_labor(
    path: web::Path<(u64, u64)>,
    form: web::Form<LaborAllocationForm>,
    pool: web::Data<DbPool>,
    event_store: web::Data<EventStore>,
) -> Result<HttpResponse> {
    let (colony_id, building_id) = path.into_inner();
    let conn = pool.get()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // 1. Get current building state
    let workers_assigned: u32 = conn.query_row(
        "SELECT workers_assigned FROM buildings WHERE building_id = ?1 AND colony_id = ?2",
        rusqlite::params![building_id, colony_id],
        |row| row.get(0)
    ).map_err(|e| actix_web::error::ErrorNotFound(format!("Building not found: {}", e)))?;

    // 2. Create Command
    let command = DeallocateLabor {
        colony_id: ColonyId(colony_id),
        building_id: BuildingId(building_id),
        workers_to_deallocate: form.workers,
        current_workers: workers_assigned,
    };

    // 3. Execute
    let events = command.execute()
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // 4. Persist Events
    for event in events {
        let game_event = GameEvent::new(
            event_store.get_next_event_id()
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
            1,
            event.clone(),
        );
        event_store.save_event(&game_event)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        // 5. Update Read Model
        if let EventType::LaborDeallocated { building_id, workers_deallocated, .. } = event {
            conn.execute(
                "UPDATE buildings SET workers_assigned = workers_assigned - ?1 WHERE building_id = ?2",
                rusqlite::params![workers_deallocated, building_id.0],
            ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        }
    }

    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", format!("/colony/{}", colony_id)))
        .finish())
}

pub async fn toggle_building(
    path: web::Path<(u64, u64)>,
    pool: web::Data<DbPool>,
    event_store: web::Data<EventStore>,
) -> Result<HttpResponse> {
    let (colony_id, building_id) = path.into_inner();
    let conn = pool.get()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // 1. Get current building state
    let state_str: String = conn.query_row(
        "SELECT state FROM buildings WHERE building_id = ?1 AND colony_id = ?2",
        rusqlite::params![building_id, colony_id],
        |row| row.get(0)
    ).map_err(|e| actix_web::error::ErrorNotFound(format!("Building not found: {}", e)))?;

    let current_state: BuildingState = serde_json::from_str(&state_str)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // 2. Determine new state
    let new_state = match current_state {
        BuildingState::Operational => BuildingState::Shutdown,
        BuildingState::Shutdown => BuildingState::Operational,
        _ => return Err(actix_web::error::ErrorBadRequest("Cannot toggle building in current state (Damaged or Under Construction)")),
    };

    // 3. Create Command
    let command = ChangeBuildingState {
        building_id: BuildingId(building_id),
        colony_id: ColonyId(colony_id),
        new_state: new_state.clone(),
    };

    // 4. Execute
    let events = command.execute()
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // 5. Persist Events
    for event in events {
        let game_event = GameEvent::new(
            event_store.get_next_event_id()
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
            1,
            event.clone(),
        );
        event_store.save_event(&game_event)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        // 6. Update Read Model
        if let EventType::BuildingStateChanged { building_id, new_state, .. } = event {
            conn.execute(
                "UPDATE buildings SET state = ?1 WHERE building_id = ?2",
                rusqlite::params![serde_json::to_string(&new_state).unwrap(), building_id.0],
            ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        }
    }

    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", format!("/colony/{}", colony_id)))
        .finish())
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
    let (type_str, state_str): (String, String) = conn.query_row(
        "SELECT building_type, state FROM buildings WHERE building_id = ?1 AND colony_id = ?2",
        rusqlite::params![building_id, colony_id],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).map_err(|e| actix_web::error::ErrorNotFound(format!("Building not found: {}", e)))?;

    let _building_type: BuildingType = serde_json::from_str(&type_str)
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

    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", format!("/colony/{}", colony_id)))
        .finish())
}
