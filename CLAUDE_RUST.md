# CLAUDE_RUST.md - Rust Best Practices for Outpost 3 Prototype

## Project Overview

This document provides Rust-specific guidelines for AI assistants working on the **Outpost 3 Game Prototype** - a web-based simulation game built with Rust, Actix-web, HTMX, and SQLite.

## Technology Stack

- **Backend**: Rust with Actix-web 4.x
- **Database**: SQLite with rusqlite and r2d2 connection pooling
- **Frontend**: HTMX with server-rendered HTML templates
- **Templating**: Tera templates
- **Architecture**: Event Sourcing with CQRS patterns
- **Build**: Cargo with workspace organization

## Project Structure

```
outpost-3/
├── Cargo.toml              # Workspace root
├── CLAUDE_RUST.md          # This file
├── DESIGN.md               # Game design document
├── ROADMAP.md              # Feature implementation checklist
├── README.md               # Setup and run instructions
├── src/                    # Main application source
│   ├── main.rs            # Application entry point
│   ├── lib.rs             # Library exports
│   ├── config.rs          # Configuration management
│   ├── domain/            # Domain models and logic
│   │   ├── mod.rs
│   │   ├── colony.rs      # Colony entity
│   │   ├── building.rs    # Building types
│   │   ├── resource.rs    # Resource types
│   │   ├── wormhole.rs    # Wormhole gates
│   │   ├── train.rs       # Train entities
│   │   └── planet.rs      # Planet generation
│   ├── events/            # Event sourcing events
│   │   ├── mod.rs
│   │   ├── event.rs       # Base event types
│   │   └── store.rs       # Event store implementation
│   ├── commands/          # Command pattern implementations
│   │   ├── mod.rs
│   │   └── handlers.rs    # Command handlers
│   ├── queries/           # CQRS query side
│   │   ├── mod.rs
│   │   └── projections.rs # Read model projections
│   ├── services/          # Application services
│   │   ├── mod.rs
│   │   ├── colony_service.rs
│   │   └── economy_service.rs
│   ├── web/               # Web layer
│   │   ├── mod.rs
│   │   ├── routes.rs      # Route definitions
│   │   ├── handlers.rs    # HTTP handlers
│   │   └── templates/     # Tera templates
│   ├── db/                # Database layer
│   │   ├── mod.rs
│   │   ├── schema.rs      # Schema definitions
│   │   └── migrations.rs  # Migration logic
│   └── utils/             # Utility functions
│       └── mod.rs
├── static/                # Static assets
│   ├── css/
│   ├── js/
│   └── images/
├── templates/             # Tera HTML templates
│   ├── base.html
│   ├── colony.html
│   └── components/
├── tests/                 # Integration tests
│   └── integration_tests.rs
└── migrations/            # SQL migration files
    └── 001_initial_schema.sql
```

## Rust Best Practices

### 1. Error Handling

**Use `thiserror` for domain errors and `anyhow` for application errors:**

```rust
// Domain errors - use thiserror
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ColonyError {
    #[error("Insufficient resources: {resource_type}")]
    InsufficientResources { resource_type: String },

    #[error("Building {building_id} not found")]
    BuildingNotFound { building_id: u64 },

    #[error("Invalid command: {0}")]
    InvalidCommand(String),
}

// Application errors - use anyhow
use anyhow::{Context, Result};

pub fn load_colony(id: u64) -> Result<Colony> {
    db::get_colony(id)
        .context("Failed to load colony from database")?
}
```

**Never use `unwrap()` or `expect()` in production code** - always handle errors properly.

### 2. Type Safety and Domain Modeling

**Use newtype patterns for domain primitives:**

```rust
// Good - type-safe IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColonyId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BuildingId(pub u64);

// Good - strongly-typed resources
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Credits(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Energy(pub u64);
```

**Use enums for variants and states:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildingType {
    Mine { resource_type: ResourceType, output_rate: u32 },
    Factory { produces: ProductType, consumes: Vec<ResourceType> },
    PowerPlant { output_mw: u32, fuel_type: FuelType },
    Housing { capacity: u32, comfort_level: u8 },
    TrainStation { platforms: u8, throughput: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingState {
    UnderConstruction { progress: u8 },
    Operational,
    Damaged { severity: u8 },
    Shutdown,
}
```

### 3. Event Sourcing Patterns

**Events should be immutable, serializable, and past-tense named:**

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub event_id: u64,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EventType {
    ColonyFounded {
        colony_id: ColonyId,
        planet_id: PlanetId,
        starting_resources: Resources,
    },
    BuildingConstructed {
        building_id: BuildingId,
        colony_id: ColonyId,
        building_type: BuildingType,
    },
    ResourcesGathered {
        colony_id: ColonyId,
        resource_type: ResourceType,
        amount: u64,
    },
    WormholeActivated {
        wormhole_id: WormholeId,
        source_planet: PlanetId,
        destination_planet: PlanetId,
    },
    TrainDispatched {
        train_id: TrainId,
        route_id: RouteId,
        cargo: Cargo,
    },
}
```

**Commands should validate before generating events:**

```rust
pub trait Command {
    type Event;
    type Error;

    fn validate(&self) -> Result<(), Self::Error>;
    fn execute(&self) -> Result<Vec<Self::Event>, Self::Error>;
}

pub struct ConstructBuilding {
    pub colony_id: ColonyId,
    pub building_type: BuildingType,
    pub location: Location,
}

impl Command for ConstructBuilding {
    type Event = EventType;
    type Error = ColonyError;

    fn validate(&self) -> Result<(), Self::Error> {
        // Check resources, space, prerequisites, etc.
        Ok(())
    }

    fn execute(&self) -> Result<Vec<Self::Event>, Self::Error> {
        self.validate()?;

        Ok(vec![
            EventType::BuildingConstructed {
                building_id: BuildingId::new(),
                colony_id: self.colony_id,
                building_type: self.building_type.clone(),
            },
            EventType::ResourcesConsumed {
                colony_id: self.colony_id,
                costs: self.building_type.construction_cost(),
            },
        ])
    }
}
```

### 4. Async and Actix-web Patterns

**Use async/await throughout the web layer:**

```rust
use actix_web::{web, HttpResponse, Result};

#[derive(Deserialize)]
pub struct BuildingForm {
    building_type: String,
    location_x: i32,
    location_y: i32,
}

pub async fn construct_building(
    colony_id: web::Path<u64>,
    form: web::Form<BuildingForm>,
    colony_service: web::Data<ColonyService>,
) -> Result<HttpResponse> {
    let command = ConstructBuilding {
        colony_id: ColonyId(*colony_id),
        building_type: parse_building_type(&form.building_type)?,
        location: Location::new(form.location_x, form.location_y),
    };

    colony_service.execute_command(command).await?;

    Ok(HttpResponse::Ok()
        .insert_header(("HX-Trigger", "buildingAdded"))
        .body("Building construction started"))
}
```

**Use `web::Data` for shared state and services:**

```rust
pub async fn configure_app(cfg: &mut web::ServiceConfig) {
    let db_pool = create_db_pool();
    let event_store = EventStore::new(db_pool.clone());
    let colony_service = ColonyService::new(event_store);

    cfg
        .app_data(web::Data::new(colony_service))
        .service(
            web::scope("/colony")
                .route("/{id}", web::get().to(get_colony))
                .route("/{id}/building", web::post().to(construct_building))
        );
}
```

### 5. Database and Connection Pooling

**Use r2d2 for connection pooling:**

```rust
use r2d2_sqlite::SqliteConnectionManager;
use r2d2::Pool;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn create_db_pool() -> DbPool {
    let manager = SqliteConnectionManager::file("outpost3.db");
    Pool::builder()
        .max_size(15)
        .build(manager)
        .expect("Failed to create database pool")
}
```

**Use prepared statements and parameterized queries:**

```rust
pub fn save_event(pool: &DbPool, event: &GameEvent) -> Result<()> {
    let conn = pool.get()?;

    conn.execute(
        "INSERT INTO events (event_id, timestamp, event_type, data) VALUES (?1, ?2, ?3, ?4)",
        params![
            event.event_id,
            event.timestamp.to_rfc3339(),
            serde_json::to_string(&event.event_type)?,
            serde_json::to_string(&event)?,
        ],
    )?;

    Ok(())
}
```

### 6. Testing

**Write unit tests for domain logic:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construct_building_insufficient_resources() {
        let command = ConstructBuilding {
            colony_id: ColonyId(1),
            building_type: BuildingType::Mine {
                resource_type: ResourceType::Iron,
                output_rate: 10,
            },
            location: Location::new(5, 5),
        };

        let result = command.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_event_serialization() {
        let event = EventType::ColonyFounded {
            colony_id: ColonyId(1),
            planet_id: PlanetId(42),
            starting_resources: Resources::default(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: EventType = serde_json::from_str(&json).unwrap();

        assert_eq!(event, deserialized);
    }
}
```

**Use integration tests for HTTP endpoints:**

```rust
#[cfg(test)]
mod integration_tests {
    use actix_web::{test, App};
    use super::*;

    #[actix_web::test]
    async fn test_get_colony() {
        let app = test::init_service(
            App::new()
                .configure(configure_app)
        ).await;

        let req = test::TestRequest::get()
            .uri("/colony/1")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
```

### 7. Code Organization

**Keep modules focused and cohesive:**

- **Domain layer**: Pure business logic, no I/O or web concerns
- **Events layer**: Event definitions and store
- **Commands layer**: Command pattern implementations
- **Services layer**: Orchestration and application logic
- **Web layer**: HTTP handlers, routing, template rendering
- **DB layer**: Database access and queries

**Use `mod.rs` to organize module exports:**

```rust
// src/domain/mod.rs
pub mod colony;
pub mod building;
pub mod resource;
pub mod wormhole;
pub mod train;
pub mod planet;

pub use colony::{Colony, ColonyId};
pub use building::{Building, BuildingType, BuildingId};
pub use resource::{Resource, ResourceType, Resources};
// ... etc
```

### 8. Performance Considerations

**Use `Cow` for potentially borrowed data:**

```rust
use std::borrow::Cow;

pub struct TemplateContext<'a> {
    pub title: Cow<'a, str>,
    pub colony_name: Cow<'a, str>,
}
```

**Prefer iterators over collecting when possible:**

```rust
// Good
let total_power: u64 = buildings
    .iter()
    .filter_map(|b| b.power_output())
    .sum();

// Less efficient
let power_plants: Vec<_> = buildings.iter()
    .filter(|b| matches!(b.building_type, BuildingType::PowerPlant { .. }))
    .collect();
let total_power: u64 = power_plants.iter()
    .map(|b| b.power_output().unwrap())
    .sum();
```

**Use `Arc` for shared immutable data:**

```rust
use std::sync::Arc;

#[derive(Clone)]
pub struct GameConfig {
    pub inner: Arc<GameConfigInner>,
}

pub struct GameConfigInner {
    pub building_costs: HashMap<BuildingType, Resources>,
    pub tech_tree: TechTree,
}
```

### 9. HTMX Integration

**Structure templates for HTMX responses:**

```html
<!-- Full page template -->
{% extends "base.html" %}
{% block content %}
<div id="colony-view" hx-get="/colony/{{ colony_id }}/refresh" hx-trigger="every 5s">
    {% include "components/colony_stats.html" %}
</div>
{% endblock %}

<!-- Partial component for HTMX swap -->
<!-- templates/components/colony_stats.html -->
<div class="stats">
    <div class="stat">
        <span class="label">Population:</span>
        <span class="value">{{ colony.population }}</span>
    </div>
    <div class="stat">
        <span class="label">Credits:</span>
        <span class="value">{{ colony.credits }}</span>
    </div>
</div>
```

**Return appropriate HTMX headers:**

```rust
pub async fn refresh_colony_stats(
    colony_id: web::Path<u64>,
    tmpl: web::Data<tera::Tera>,
    service: web::Data<ColonyService>,
) -> Result<HttpResponse> {
    let colony = service.get_colony(ColonyId(*colony_id)).await?;

    let mut context = tera::Context::new();
    context.insert("colony", &colony);

    let html = tmpl.render("components/colony_stats.html", &context)?;

    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(html))
}
```

### 10. Logging and Debugging

**Use `tracing` for structured logging:**

```rust
use tracing::{info, warn, error, debug, instrument};

#[instrument(skip(service))]
pub async fn execute_command(
    command: ConstructBuilding,
    service: &ColonyService,
) -> Result<()> {
    debug!("Validating command");
    command.validate()?;

    info!(colony_id = ?command.colony_id, building_type = ?command.building_type,
          "Executing building construction command");

    let events = command.execute()?;

    for event in events {
        service.apply_event(event).await?;
        debug!(event = ?event, "Event applied");
    }

    info!("Command executed successfully");
    Ok(())
}
```

 make sure to write UI code in way that logs events and other data indicating when and how things are drawn. also log state changes to help with testing and debugging

### 11. Serialization

**Use serde with appropriate attributes:**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Colony {
    pub id: ColonyId,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default)]
    pub resources: Resources,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub founded_at: DateTime<Utc>,
}
```

### 12. Common Patterns to Avoid

**Don't use `clone()` excessively - prefer borrowing:**

```rust
// Bad
fn process_colony(colony: Colony) -> Colony {
    let mut updated = colony.clone();
    updated.process();
    updated
}

// Good
fn process_colony(colony: &mut Colony) {
    colony.process();
}
```

**Don't use `String` when `&str` suffices:**

```rust
// Bad
fn get_building_name(building_type: String) -> String {
    format!("Building: {}", building_type)
}

// Good
fn get_building_name(building_type: &str) -> String {
    format!("Building: {}", building_type)
}
```

**Don't ignore errors with `let _ =`:**

```rust
// Bad
let _ = save_to_database(&data);

// Good
if let Err(e) = save_to_database(&data) {
    error!("Failed to save to database: {}", e);
}
```

## Dependencies to Use

### Core Dependencies
```toml
[dependencies]
actix-web = "4"
actix-rt = "2"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
tera = "1"
thiserror = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
rusqlite = { version = "0.31", features = ["bundled"] }
r2d2 = "0.8"
r2d2_sqlite = "0.24"
uuid = { version = "1", features = ["v4", "serde"] }
rand = "0.8"
```

### Development Dependencies
```toml
[dev-dependencies]
actix-web-test = "4"
```

## Common Tasks

### Adding a New Domain Entity
1. Create file in `src/domain/`
2. Define struct with newtype IDs
3. Implement business logic methods
4. Add to `src/domain/mod.rs` exports
5. Create related events in `src/events/`
6. Create related commands in `src/commands/`
7. Write unit tests

### Adding a New Screen/Route
1. Create handler in `src/web/handlers.rs`
2. Add route to `src/web/routes.rs`
3. Create Tera template in `templates/`
4. Add HTMX attributes for interactivity
5. Create CSS in `static/css/` if needed
6. Write integration test

### Adding a New Command
1. Define command struct in `src/commands/`
2. Implement `Command` trait with validation
3. Define events it generates
4. Add handler to web layer if needed
5. Write tests for validation and execution

## AI Assistant Guidelines

When working on this codebase:

1. **Always use proper error handling** - no `unwrap()` or `expect()`
2. **Follow the event sourcing pattern** - state changes go through commands and events
3. **Keep domain logic pure** - no I/O in domain layer
4. **Use strong typing** - newtype wrappers for IDs and domain primitives
5. **Write tests** - unit tests for domain, integration tests for web
6. **Keep it simple** - avoid over-engineering, focus on working features
7. **Document complex logic** - use doc comments for public APIs
8. **Follow Rust idioms** - prefer iterators, pattern matching, and Result types
9. **Structure for HTMX** - return HTML fragments for dynamic updates
10. **Log appropriately** - use tracing for debugging and monitoring

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Actix-web Documentation](https://actix.rs/)
- [HTMX Documentation](https://htmx.org/)
- [Tera Template Guide](https://tera.netlify.app/)
- [Event Sourcing with Rust](https://github.com/serverlesstechnology/cqrs)
