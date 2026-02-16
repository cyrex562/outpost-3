use anyhow::{Context, Result};
use rusqlite::params;
use tracing::info;

use super::{EventType, GameEvent};
use crate::db::pool::DbPool;

pub struct EventStore {
    pool: DbPool,
}

impl EventStore {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn save_event(&self, event: &GameEvent) -> Result<()> {
        let conn = self
            .pool
            .get()
            .context("Failed to get database connection")?;

        let event_data =
            serde_json::to_string(&event.event_type).context("Failed to serialize event")?;

        // Log the event with structured fields
        Self::log_event(&event.event_type, event.event_id, event.turn_number);

        conn.execute(
            "INSERT INTO events (event_id, timestamp, turn_number, event_type, event_data) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                event.event_id,
                event.timestamp.to_rfc3339(),
                event.turn_number,
                serde_json::to_string(&event.event_type).unwrap(),
                event_data,
            ],
        ).context("Failed to insert event")?;

        Ok(())
    }

    pub fn get_all_events(&self) -> Result<Vec<GameEvent>> {
        let conn = self
            .pool
            .get()
            .context("Failed to get database connection")?;

        let mut stmt = conn.prepare(
            "SELECT event_id, timestamp, turn_number, event_data FROM events ORDER BY event_id ASC",
        )?;

        let events = stmt
            .query_map([], |row| {
                let event_id: u64 = row.get(0)?;
                let timestamp_str: String = row.get(1)?;
                let turn_number: u64 = row.get(2)?;
                let event_data: String = row.get(3)?;

                let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap();

                let event_type = serde_json::from_str(&event_data).unwrap();

                Ok(GameEvent {
                    event_id,
                    timestamp,
                    turn_number,
                    event_type,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    pub fn get_next_event_id(&self) -> Result<u64> {
        let conn = self
            .pool
            .get()
            .context("Failed to get database connection")?;

        let count: u64 = conn.query_row(
            "SELECT COALESCE(MAX(event_id), 0) + 1 FROM events",
            [],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// Log event with structured fields based on event type
    fn log_event(event_type: &EventType, event_id: u64, turn_number: u64) {
        match event_type {
            // Colony events
            EventType::ColonyFounded {
                colony_id,
                planet_id,
                name,
                ..
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    colony_id = ?colony_id,
                    planet_id = ?planet_id,
                    name = %name,
                    "Colony founded"
                );
            }
            EventType::BuildingConstructionStarted {
                building_id,
                colony_id,
                building_type,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    building_id = ?building_id,
                    colony_id = ?colony_id,
                    building_type = ?building_type,
                    "Building construction started"
                );
            }
            EventType::BuildingConstructionCompleted {
                building_id,
                colony_id,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    building_id = ?building_id,
                    colony_id = ?colony_id,
                    "Building construction completed"
                );
            }
            EventType::BuildingStateChanged {
                building_id,
                colony_id,
                new_state,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    building_id = ?building_id,
                    colony_id = ?colony_id,
                    new_state = ?new_state,
                    "Building state changed"
                );
            }
            EventType::ResourcesExtracted {
                colony_id,
                resource_type,
                amount,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    colony_id = ?colony_id,
                    resource_type = ?resource_type,
                    amount = amount,
                    "Resources extracted"
                );
            }
            EventType::ResourcesConsumed {
                colony_id,
                resource_type,
                amount,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    colony_id = ?colony_id,
                    resource_type = ?resource_type,
                    amount = amount,
                    "Resources consumed"
                );
            }
            EventType::ResourcesProduced {
                colony_id,
                resource_type,
                amount,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    colony_id = ?colony_id,
                    resource_type = ?resource_type,
                    amount = amount,
                    "Resources produced"
                );
            }
            EventType::PopulationChanged {
                colony_id,
                new_population,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    colony_id = ?colony_id,
                    new_population = new_population,
                    "Population changed"
                );
            }
            EventType::PowerGridUpdated {
                colony_id,
                generation,
                consumption,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    colony_id = ?colony_id,
                    generation = generation,
                    consumption = consumption,
                    deficit_or_surplus = *generation as i64 - *consumption as i64,
                    "Power grid updated"
                );
            }
            EventType::PopulationGrew {
                colony_id,
                old_population,
                new_population,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    colony_id = ?colony_id,
                    old_population = old_population,
                    new_population = new_population,
                    growth = new_population - old_population,
                    "Population grew"
                );
            }
            EventType::LaborAllocated {
                colony_id,
                building_id,
                workers_allocated,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    colony_id = ?colony_id,
                    building_id = ?building_id,
                    workers_allocated = workers_allocated,
                    "Labor allocated"
                );
            }
            EventType::LaborDeallocated {
                colony_id,
                building_id,
                workers_deallocated,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    colony_id = ?colony_id,
                    building_id = ?building_id,
                    workers_deallocated = workers_deallocated,
                    "Labor deallocated"
                );
            }
            EventType::BuildingDamaged {
                building_id,
                colony_id,
                damage_severity,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    building_id = ?building_id,
                    colony_id = ?colony_id,
                    damage_severity = damage_severity,
                    "Building damaged"
                );
            }
            EventType::BuildingRepaired {
                building_id,
                colony_id,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    building_id = ?building_id,
                    colony_id = ?colony_id,
                    "Building repaired"
                );
            }
            EventType::BuildingUpgraded {
                building_id,
                colony_id,
                new_level,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    building_id = ?building_id,
                    colony_id = ?colony_id,
                    new_level = new_level,
                    "Building upgraded"
                );
            }
            EventType::BuildingRecipeChanged {
                building_id,
                colony_id,
                old_recipe_index,
                new_recipe_index,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    building_id = ?building_id,
                    colony_id = ?colony_id,
                    old_recipe_index = old_recipe_index,
                    new_recipe_index = new_recipe_index,
                    "Building recipe changed"
                );
            }
            // Market events
            EventType::MarketPriceChanged {
                resource,
                old_price,
                new_price,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    resource = ?resource,
                    old_price = old_price,
                    new_price = new_price,
                    change_pct = ((new_price - old_price) / old_price * 100.0),
                    "Market price changed"
                );
            }
            // Trading events
            EventType::ResourceTraded {
                colony_id,
                resource_type,
                quantity,
                price,
                side,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    colony_id = ?colony_id,
                    resource_type = ?resource_type,
                    quantity = quantity,
                    price = price,
                    side = ?side,
                    total_cost = (*quantity as f64 * price),
                    "Resource traded"
                );
            }
            // Economy events
            EventType::EconomySnapshot {
                colony_id,
                gdp,
                income,
                expenses,
                net_worth,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    colony_id = ?colony_id,
                    gdp = gdp,
                    income = income,
                    expenses = expenses,
                    net_worth = net_worth,
                    "Economy snapshot"
                );
            }
            // Banking events
            EventType::LoanIssued {
                loan_id,
                colony_id,
                principal,
                interest_rate,
                term_turns,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    loan_id = ?loan_id,
                    colony_id = ?colony_id,
                    principal = principal,
                    interest_rate = interest_rate,
                    term_turns = term_turns,
                    "Loan issued"
                );
            }
            EventType::LoanPaymentMade {
                loan_id,
                colony_id,
                amount,
                principal_paid,
                interest_paid,
                remaining_principal,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    loan_id = ?loan_id,
                    colony_id = ?colony_id,
                    amount = amount,
                    principal_paid = principal_paid,
                    interest_paid = interest_paid,
                    remaining_principal = remaining_principal,
                    "Loan payment made"
                );
            }
            EventType::LoanPaidOff { loan_id, colony_id } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    loan_id = ?loan_id,
                    colony_id = ?colony_id,
                    "Loan paid off"
                );
            }
            // Wormhole events
            EventType::PlanetDiscovered { planet_id, .. } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    planet_id = ?planet_id,
                    "Planet discovered"
                );
            }
            EventType::GateConstructionStarted {
                gate_id,
                source_planet,
                destination_planet,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    gate_id = ?gate_id,
                    source_planet = ?source_planet,
                    destination_planet = ?destination_planet,
                    "Gate construction started"
                );
            }
            EventType::GateConstructionCompleted { gate_id } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    gate_id = ?gate_id,
                    "Gate construction completed"
                );
            }
            EventType::GateActivated { gate_id } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    gate_id = ?gate_id,
                    "Gate activated"
                );
            }
            EventType::GateDeactivated { gate_id } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    gate_id = ?gate_id,
                    "Gate deactivated"
                );
            }
            // Train events
            EventType::TrainPurchased {
                train_id,
                colony_id,
                train_type,
                cost,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    train_id = ?train_id,
                    colony_id = ?colony_id,
                    train_type = ?train_type,
                    cost = cost,
                    "Train purchased"
                );
            }
            EventType::TrainAssignedToRoute { train_id, route_id } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    train_id = ?train_id,
                    route_id = ?route_id,
                    "Train assigned to route"
                );
            }
            EventType::TrainDispatched {
                train_id,
                route_id,
                origin,
                destination,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    train_id = ?train_id,
                    route_id = ?route_id,
                    origin = ?origin,
                    destination = ?destination,
                    "Train dispatched"
                );
            }
            EventType::TrainArrived {
                train_id,
                planet_id,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    train_id = ?train_id,
                    planet_id = ?planet_id,
                    "Train arrived"
                );
            }
            // Rail events
            EventType::RailConstructionStarted {
                segment_id,
                colony_id,
                from,
                to,
                rail_type,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    segment_id = ?segment_id,
                    colony_id = ?colony_id,
                    from = ?from,
                    to = ?to,
                    rail_type = ?rail_type,
                    "Rail construction started"
                );
            }
            EventType::RailConstructionCompleted {
                segment_id,
                colony_id,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    segment_id = ?segment_id,
                    colony_id = ?colony_id,
                    "Rail construction completed"
                );
            }
            EventType::RailUpgraded {
                segment_id,
                colony_id,
                new_type,
                new_count,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    segment_id = ?segment_id,
                    colony_id = ?colony_id,
                    new_type = ?new_type,
                    new_count = new_count,
                    "Rail upgraded"
                );
            }
            EventType::RailRemoved {
                segment_id,
                colony_id,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    segment_id = ?segment_id,
                    colony_id = ?colony_id,
                    "Rail removed"
                );
            }
            EventType::StationBuilt {
                station_id,
                colony_id,
                building_id,
                connected_segments,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    station_id = ?station_id,
                    colony_id = ?colony_id,
                    building_id = ?building_id,
                    connected_segments = ?connected_segments,
                    "Station built"
                );
            }
            EventType::StationRemoved {
                station_id,
                colony_id,
            } => {
                info!(
                    event_id = event_id,
                    turn = turn_number,
                    station_id = ?station_id,
                    colony_id = ?colony_id,
                    "Station removed"
                );
            }
            // Simulation events
            EventType::TurnAdvanced { turn_number: turn } => {
                info!(event_id = event_id, turn = turn, "Turn advanced");
            }
        }
    }
}
