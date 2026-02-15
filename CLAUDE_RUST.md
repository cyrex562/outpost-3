# CLAUDE_RUST.md — Rust Web Dev Guide for Outpost 3

**For:** AI coding assistants (Claude, Copilot, Gemini, Cursor, etc.)
**Project:** Outpost 3 — colony-building simulation game
**Design:** `docs/Outpost3_Design_V5.md` (V5, the single source of truth for game design)
**Checklist:** `docs/MVP_TRANSFORMATION_CHECKLIST.md` (tracks every task to reach MVP)

---

## You Are a Rust Web Developer

You are working on a **Rust web application**. Your role is that of a **senior Rust web developer** building a data-driven simulation game served as a server-rendered web app. You write idiomatic, well-tested Rust. You iterate until **all unit, property, and integration tests pass**.

---

## Technology Stack

| Layer | Technology | Notes |
|---|---|---|
| **Core Logic** | Rust (pure, no I/O) | `outpost-core` crate — all simulation, domain, events, commands |
| **Web Server** | Actix-Web 4.x | `outpost-server` crate — HTTP, templates, DB, static files |
| **Database** | SQLite + rusqlite + r2d2 | Connection pooling, prepared statements, parameterized queries |
| **Templates** | Tera | Server-side HTML rendering |
| **Frontend** | HTMX + Alpine.js | Partial page updates, lightweight client-side interactivity |
| **Content** | YAML data files | Buildings, resources, recipes, events, tech tree loaded at startup |
| **Testing** | `#[test]` + `proptest` | Unit tests, property-based tests, integration tests |
| **Error Handling** | `thiserror` (domain) + `anyhow` (application) | Never `unwrap()` or `expect()` in production code |
| **Logging** | `tracing` + `tracing-subscriber` | Structured logging in server code only (not in core) |

---

## Project Structure (Target — V5 Architecture)

```
outpost-3/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── outpost-core/           # Pure game logic (NO I/O, NO web deps)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── domain/         # Entity structs, game state, business logic
│   │       ├── events/         # Event types (immutable, past-tense)
│   │       ├── commands/       # Command structs (validate → produce events)
│   │       ├── simulation/     # Tick processing, game clock
│   │       ├── content/        # YAML content loader and definitions
│   │       └── errors.rs       # Domain error types (thiserror)
│   └── outpost-server/         # Web server (Actix, DB, templates)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs         # Entry point, server startup
│           ├── config.rs       # AppConfig loading
│           ├── routes.rs       # Route definitions
│           ├── handlers/       # HTTP handler functions
│           ├── db/             # SQLite schema, migrations, queries
│           └── services/       # Orchestration between core and DB
├── content/                    # YAML data files (buildings, resources, recipes, events)
├── templates/                  # Tera HTML templates
│   ├── base.html              # Master layout (sidebar + topbar + content + ticker)
│   ├── dashboard.html
│   ├── site_detail.html
│   ├── colonies.html
│   ├── events.html
│   ├── settings.html
│   └── components/            # Partial templates for HTMX swaps
├── static/
│   ├── css/                   # Stylesheets (dark theme, data-dense tables)
│   └── js/                    # Alpine.js components, HTMX config
├── tests/                     # Integration tests
└── docs/
    ├── Outpost3_Design_V5.md  # Game design document (source of truth)
    └── MVP_TRANSFORMATION_CHECKLIST.md
```

### Critical Rule: `outpost-core` Has Zero I/O

The `outpost-core` crate must **never** depend on:
- `actix-web`, `tokio`, or any async runtime
- `rusqlite`, `r2d2`, or any database crate
- `tracing` (use return values, not logging, to communicate state)
- File system access, network calls, or any side effects

All I/O happens in `outpost-server`. Core is pure functions: `(State, Command) → (State', Events[])`.

---

## Architecture: Event Sourcing + CQRS

### The Pattern

1. **Player action** → HTTP handler receives request
2. **Handler** creates a **Command** and passes it to a service
3. **Service** calls `command.execute(&current_state)` in `outpost-core`
4. **Command** validates against state, returns `Vec<Event>` or error
5. **Service** applies events to state, persists events to DB
6. **UI** reads projected state (query side) and renders templates

### Events (Past-Tense, Immutable)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GameEvent {
    SiteFounded { site_id: SiteId, body_id: BodyId, name: String },
    BuildingQueued { site_id: SiteId, building_def_id: String, job_id: JobId },
    BuildingCompleted { site_id: SiteId, building_id: BuildingId },
    ResourcesExtracted { site_id: SiteId, resource: String, amount: f64 },
    ResourcesConsumed { site_id: SiteId, resource: String, amount: f64 },
    PopulationChanged { site_id: SiteId, delta: i64, reason: String },
    MoraleChanged { site_id: SiteId, new_value: f64, factors: Vec<String> },
    TickProcessed { tick: u64 },
}
```

### Commands (Present-Tense, Validate → Execute)

```rust
pub trait Command {
    fn execute(&self, state: &GameState) -> Result<Vec<GameEvent>, DomainError>;
}

pub struct ConstructBuilding {
    pub site_id: SiteId,
    pub building_def_id: String,  // references YAML content
}

impl Command for ConstructBuilding {
    fn execute(&self, state: &GameState) -> Result<Vec<GameEvent>, DomainError> {
        let site = state.get_site(self.site_id)?;
        let def = state.content.get_building(&self.building_def_id)?;

        // Validate: resources available, labor available, prerequisites met
        site.validate_resources(&def.construction_cost)?;
        site.validate_labor(def.construction_labor)?;

        Ok(vec![
            GameEvent::BuildingQueued {
                site_id: self.site_id,
                building_def_id: self.building_def_id.clone(),
                job_id: JobId::new(),
            },
            GameEvent::ResourcesConsumed {
                site_id: self.site_id,
                resource: "construction_materials".into(),
                amount: def.construction_cost.total(),
            },
        ])
    }
}
```

---

## Data-Driven Content (YAML)

Buildings, resources, recipes, and events are defined in YAML files under `content/`. The game loads these at startup via a `ContentLoader` in `outpost-core`.

```yaml
# content/buildings.yaml
- id: mine
  name: "Surface Mine"
  category: industrial
  construction_cost:
    structural_components: 50
    machine_parts: 20
  construction_time_ticks: 24
  labor_slots: 5
  power_consumption: 10
  recipes:
    - id: mine_iron
      inputs: { labor: 3, power: 10 }
      outputs: { iron_ore: 15 }
      ticks: 1
```

```yaml
# content/resources.yaml
- id: iron_ore
  name: "Iron Ore"
  category: raw
  tier: 1
  storage_type: bulk
  unit: "tonnes"
```

---

## Entity Hierarchy

```
GameState
├── GameClock (tick, speed, paused)
├── Content (loaded YAML definitions)
├── Galaxy
│   └── StarSystem
│       └── CelestialBody (planet, moon, asteroid)
│           ├── ResourceDeposit[]
│           └── Site (settlement or installation)
│               ├── BuildingList[]
│               ├── ConstructionQueue[]
│               ├── ResourceStockpile (HashMap<String, f64>)
│               ├── Population (aggregate + representative characters)
│               ├── PowerGrid (generation, consumption, net)
│               ├── LifeSupport (oxygen, water, temperature)
│               ├── LaborPool (available workers by skill)
│               └── Morale (composite score 0-100)
└── EventLog (fired gameplay events)
```

---

## Type Safety

**Use newtype wrappers for all IDs:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SiteId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BuildingId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BodyId(pub Uuid);
```

**Use enums for finite sets:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingState {
    UnderConstruction,
    Operational,
    Paused,
    Damaged,
    Destroyed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SiteType {
    Settlement,
    Installation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Skill {
    Laborer, Engineer, Scientist, Farmer, Medic, Operator,
}
```

---

## Error Handling

```rust
// Domain errors in outpost-core (thiserror)
#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Site {0} not found")]
    SiteNotFound(SiteId),

    #[error("Insufficient {resource}: need {needed}, have {available}")]
    InsufficientResource { resource: String, needed: f64, available: f64 },

    #[error("Content definition '{0}' not found")]
    ContentNotFound(String),

    #[error("Building {0} is not operational")]
    BuildingNotOperational(BuildingId),
}

// Application errors in outpost-server (anyhow)
pub async fn handle_build(/* ... */) -> Result<HttpResponse> {
    let events = command.execute(&state)
        .context("Failed to execute build command")?;
    // ...
}
```

**Rules:**
- `unwrap()` and `expect()` are forbidden in non-test code
- Use `?` for propagation
- Add `.context()` at application boundaries
- Log errors with `tracing::error!` in the server crate

---

## Testing Strategy — Iterate Until All Tests Pass

Every change you make must be followed by running tests. **Do not consider a task done until all tests pass.**

### Unit Tests (in `outpost-core`)

Test pure domain logic, commands, and simulation:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_building_rejects_insufficient_resources() {
        let state = GameState::new_test();  // helper with minimal valid state
        let cmd = ConstructBuilding { site_id: state.first_site(), building_def_id: "mine".into() };
        // Site has no resources
        assert!(matches!(cmd.execute(&state), Err(DomainError::InsufficientResource { .. })));
    }

    #[test]
    fn tick_advances_construction_progress() {
        let mut state = GameState::new_test_with_construction();
        let events = state.process_tick();
        // Construction should have progressed
        let job = state.first_site_state().construction_queue.first().unwrap();
        assert!(job.progress > 0);
    }
}
```

### Property Tests (in `outpost-core`, using `proptest`)

Test invariants that must hold for all inputs:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn morale_always_in_bounds(
        food in 0.0f64..=1.0,
        water in 0.0f64..=1.0,
        housing in 0.0f64..=1.0,
    ) {
        let morale = calculate_morale(food, water, housing);
        prop_assert!(morale >= 0.0 && morale <= 100.0);
    }

    #[test]
    fn resource_conservation_in_recipes(
        input_amount in 1.0f64..1000.0,
    ) {
        // Total mass/value in = total mass/value out (within recipe ratio)
        let recipe = get_test_recipe();
        let output = execute_recipe(&recipe, input_amount);
        prop_assert!((output.total_value() - input_amount * recipe.ratio()).abs() < 0.001);
    }

    #[test]
    fn save_load_roundtrip(state in arb_game_state()) {
        let json = serde_json::to_string(&state).unwrap();
        let restored: GameState = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(state, restored);
    }
}
```

### Integration Tests (in `tests/` or `outpost-server`)

Test HTTP endpoints end-to-end:

```rust
#[actix_web::test]
async fn test_build_endpoint_creates_construction_job() {
    let app = test::init_service(create_test_app()).await;

    let req = test::TestRequest::post()
        .uri("/site/test-site-id/build")
        .set_form(&BuildForm { building_def_id: "mine".into() })
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    // Verify construction queue contains the new job
}
```

### Workflow: Iterate Until Green

1. Write or modify code
2. Run `cargo test --workspace`
3. If tests fail, read the failure output, fix the issue, go to step 2
4. Run `cargo clippy --workspace -- -D warnings`
5. If clippy warns, fix, go to step 2
6. Only then consider the task complete

---

## HTMX Integration

### Full Page vs. Partial Responses

```rust
// Full page: renders within base.html layout
pub async fn site_detail(/* ... */) -> Result<HttpResponse> {
    let html = tmpl.render("site_detail.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// Partial: returns just the component HTML (for HTMX swaps)
pub async fn site_buildings_tab(/* ... */) -> Result<HttpResponse> {
    let html = tmpl.render("components/buildings_table.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
```

### HTMX Patterns in Templates

```html
<!-- Tab navigation with HTMX -->
<div class="tabs">
  <button hx-get="/site/{{ site.id }}/buildings" hx-target="#tab-content">Buildings</button>
  <button hx-get="/site/{{ site.id }}/resources" hx-target="#tab-content">Resources</button>
  <button hx-get="/site/{{ site.id }}/labor" hx-target="#tab-content">Labor</button>
</div>
<div id="tab-content">
  {% include "components/buildings_table.html" %}
</div>

<!-- Auto-refresh stats via polling -->
<div id="resource-summary" hx-get="/site/{{ site.id }}/resources/summary" hx-trigger="every 2s">
  {% include "components/resource_summary.html" %}
</div>

<!-- Action form -->
<form hx-post="/site/{{ site.id }}/build" hx-target="#construction-queue" hx-swap="innerHTML">
  <select name="building_def_id">
    {% for b in available_buildings %}
    <option value="{{ b.id }}">{{ b.name }} ({{ b.cost_summary }})</option>
    {% endfor %}
  </select>
  <button type="submit">Build</button>
</form>
```

### UI Design Principles (V5)

- **Text-and-tables first** — no canvas, no maps, no sprites
- **Dark mode** default, high contrast, monospace numbers
- **Information density** — pack data into tables and lists
- **Sidebar + top bar + content + event ticker** layout on every page
- **HTMX for interactivity** — partial page updates, no full reloads
- **Alpine.js for client-side** — dropdowns, toggles, modals, collapsible sections
- **Tooltips** on hover for detailed breakdowns
- **Color coding** — consistent palette for resources, severity, status

---

## Common Tasks

### Adding a New Domain Entity

1. Define struct in `outpost-core/src/domain/` with newtype ID
2. Add serde derives (`Serialize`, `Deserialize`, `Debug`, `Clone`)
3. Implement business logic as methods (pure, no I/O)
4. Export from `domain/mod.rs`
5. Define related events (past-tense) in `events/`
6. Define related commands (validate + execute) in `commands/`
7. Write unit tests for all logic paths
8. Write property tests for invariants
9. Run `cargo test --workspace` — iterate until green

### Adding a New Page/Route

1. Create handler in `outpost-server/src/handlers/`
2. Add route in `outpost-server/src/routes.rs`
3. Create Tera template in `templates/` (extend `base.html`)
4. Create partial templates in `templates/components/` for HTMX
5. Add CSS in `static/css/` if needed
6. Write integration test for the endpoint
7. Run `cargo test --workspace` — iterate until green

### Adding a New Building/Resource/Event (Content)

1. Add definition to appropriate YAML file in `content/`
2. Ensure `ContentLoader` validates the new entry (run tests)
3. If new fields are needed, update the content structs in `outpost-core/src/content/`
4. Update any templates that display this content type
5. Run `cargo test --workspace` — iterate until green

---

## Key Rules for AI Assistants

1. **Read `docs/Outpost3_Design_V5.md` before starting work** — it is the authoritative design
2. **Read `docs/MVP_TRANSFORMATION_CHECKLIST.md`** — it tracks what needs to be done
3. **Keep `outpost-core` pure** — zero I/O, zero side effects
4. **All state changes go through commands and events** — never mutate state directly
5. **All entity IDs use newtype wrappers** — never raw `u64` or `String` for IDs
6. **No `unwrap()` or `expect()` in production code**
7. **Write tests first or alongside code** — unit, property, and integration
8. **Iterate until all tests pass** — `cargo test --workspace` must be green before you stop
9. **Run `cargo clippy --workspace -- -D warnings`** — fix all warnings
10. **Data-driven content** — buildings, resources, recipes, events come from YAML, not hardcoded enums
11. **Text-and-tables UI** — no canvas, no maps, no sprites, no Pixi.js
12. **HTMX for dynamic updates** — return HTML fragments, use `hx-` attributes
13. **Log with `tracing`** in server code — structured, leveled, with context spans
14. **Keep changes minimal and focused** — one feature at a time, matching the checklist
