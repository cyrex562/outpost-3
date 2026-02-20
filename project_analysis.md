# Outpost 3: Project Architecture & Analysis

## 1. Project Overview & Vision
Outpost 3 is a deeply simulated colony-building strategy game played via a data-dense, text-and-tables web interface. The player acts as a strategic director managing planetary settlements, resource chains, power grids, and orbital infrastructure, incrementally expanding across star systems via wormhole gates.

The goal of the current V5 architecture pivot is to scale from an Actix-web prototype into a pristine, modular "Server-Rendered SPA". It uses HTMX for seamless partial page updates and Alpine.js for client-side interactivity, avoiding heavy canvas/WebGL implementations in favor of data-rich management dashboards.

## 2. Core Architecture (V5)
The project strictly enforces a pure Domain-Driven, Event-Sourced architecture. It is divided into an uncompromising two-crate workspace structure:

### `outpost-core` (Pure Game Logic)
*   **Zero I/O Rule**: This crate contains NO dependencies on `actix-web`, async runtimes, databases, or logging libraries. It relies entirely on pure functions.
*   **Event Sourcing & CQRS**: Player actions flow backwards into commands (`Command` trait), which are validated against the current `GameState` to produce a `Vec<GameEvent>`. The state is then reconstructed by replaying these immutable events.
*   **Simulation Loop**: Follows a deterministic tick processor (`GameClock` and `tick_processor`).
*   **Data-Driven Content Engine**: Content (Buildings, Resources, Events, Tech) is defined in YAML and validated via a `ContentLoader` on startup.

### `outpost-server` (Web & Infrastructure)
*   **Web Framework**: Powered by Actix-Web exposing HTTP REST and HTMX-targeted endpoints.
*   **Persistence Layer**: SQLite with `rusqlite` + `r2d2` connection pooling. 
*   **Template Engine**: Tera templates rendering server-side HTML.
*   **Frontend**: Alpine.js for reactive UI state (like collapsible sidebars and modest interactivity) + HTMX for partial DOM swaps (e.g., dynamically updating the construction queue or resource dashboards).

## 3. Technology Stack
*   **Backend**: Rust (1.70.0+), Actix-Web 4, Tokio, Serde, Config, Uuid.
*   **Database**: SQLite embedded (via `rusqlite` + `r2d2`), Event Sourcing.
*   **Frontend**: HTMX, Alpine.js, Tera templates, Vanilla CSS (Custom Design System with CSS variables).
*   **Testing**: Rust standard `#[test]`, `proptest` (property-based testing for domain invariants).

## 4. Current State of Web UI and API
The team has rapidly migrated away from an older, asset-heavy prototype (Pre-V5 scope like trains, manual cargo transfers, Pixi.js maps) toward the V5 text-and-tables SPA. 

**Completed & Working:**
1.  **UI Shell Framework**: The core dashboard layout is built using a dark-theme CSS design system. It includes a collapsible sidebar, a top bar with real-time clock and speed controls (polling `/api/time/status`), and an event log ticker.
2.  **Basic Endpoints**: Routes map to detailed view handlers (`GET /colonies`, `GET /dashboard`, `GET /site/{id}`).
3.  **Site & Building Operations**: 
    *   Players can view settlements, queue construction (`POST /site/{id}/build`), pause/cancel jobs, and fetch refreshed HTMX partials for buildings and queues.
    *   10 core buildings and 26 resources have been modeled in YAML and successfully loaded into the engine.

## 5. Next Steps for Current Web UI & API Iteration
To complete the active phase (Production Chains, Population, Power & Life Support) and fully realize the V5 MVP, the following steps are required:

### Step 1: Finish Production Chains & Storage (MVP.3)
*   **Core Logic**: Integrate `process_building_production()` inside `GameState::process_tick()`. Ensure factories consume raw materials and output refined goods each tick.
*   **Storage API**: Implement dynamic storage capacity calculation aggregated from constructed warehouse/storage buildings (currently hardcoded).
*   **Web UI - Resources Tab**: 
    *   Flesh out the `Site Detail — Resources` tab template with HTMX partials.
    *   Inject tooltips showing which buildings consume/produce specific resources.
    *   Add HTMX polling to auto-refresh the global resource summary in the top bar.

### Step 2: Population & Labor Management (MVP.4)
*   **Core Logic**: Implement aggregate population on Sites, including demographic breakdowns, needs calculators (Food, Water, Oxygen), and a Morale tracker (0-100 scale). Create the `RepresentativeCharacter` struct.
*   **Labor API**: Establish `AssignLabor` and `DeallocateLabor` commands allowing players to assign workers to slots, affecting building efficiency.
*   **Web UI - Labor Tab**:
    *   Build the HTMX templates for the `Site Detail — Labor` tab.
    *   Implement sliders/controls to assign unassigned colonists to various building roles.
    *   Feed morale and population growth gauges back to the core Dashboard.

### Step 3: Power Grid & Life Support Integration (MVP.5)
*   **Core Logic**: Implement individual `PowerGrid` net calculations (generation vs. consumption). Build the brownout mechanic (buildings lose efficiency if the grid runs a deficit).
*   **Life Support API**: Track oxygen, water, and temperature requirements for habitats.
*   **Web UI Updates**:
    *   Expose `ToggleBuildingPower` as an endpoint so players can manually offline non-essential buildings during brownouts.
    *   Update Site Overview dashboards to show Life Support and Power surplus/deficit warnings.

### Step 4: Verification & Testing
*   Write integration tests verifying HTMX payload correctness for the Dashboard and newly built tabs (Resources, Labor).
*   Build out `proptest` cases verifying resource conservation (input == output + waste) during production chain ticks.
