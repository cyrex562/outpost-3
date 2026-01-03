# Desktop/WASM Pivot Plan (Phase 3.5)

Status: In Progress
Last Updated: 2026-01-03 08:35

This document captures the detailed checklist for pivoting Phase 3.5 from a web-first UI to a desktop-first application that also compiles to WebAssembly (WASM). Targets: Windows and Linux (first-class), then WASM for modern desktop browsers. No webviews; UI uses Bevy + bevy_egui and charts via egui_plot.

## Tech Stack
- [x] Bevy (2D rendering, input, assets)
- [x] bevy_egui (immediate-mode UI)
- [x] egui_plot (charts)
- [x] serde + RON or JSON for saves (desktop)
- [x] IndexedDB for WASM storage (via gloo or indexed_db_futures)

## Architecture Overview
- [x] Create crate: outpost-core (simulation, domain, event sourcing, queries/commands API; no platform I/O)
- [x] Create crate: outpost-client (Bevy + bevy_egui UI; consumes outpost-core)
- [x] Storage abstraction trait with desktop (file) and WASM (IndexedDB) implementations
- [x] Feature flags: desktop and wasm to switch platform specifics

## Milestone 0 — Scaffolding & Proof Harness
- [x] outpost-core: define minimal types GameState, Command, Event, AdvanceTurnResult
- [x] outpost-core: implement advance_turn(state) -> AdvanceTurnResult
- [x] outpost-core: expose read-only query structs for resources, power, population
- [x] outpost-client: add Bevy + bevy_egui dependencies
- [x] outpost-client: boot a window and show a basic egui panel (FPS + button)
- [x] outpost-client: define Storage trait and provide desktop/WASM stubs
- [x] Wire outpost-core as a dependency of outpost-client
- [x] Docs: update README with quick start for desktop and WASM runs
- [x] CI: add minimal GitHub Actions workflow for Windows/Linux builds

## Milestone 1 — Hex Map Foundation
- [x] Implement axial hex math (Hex { q, r }, neighbor, ring, line, transforms)
- [x] Camera: pan (drag or WASD), zoom (wheel) with clamped limits
- [x] Render 50x50 hex grid with terrain coloring
- [x] Selection: hover highlight, click to select, tooltip overlay (egui)
- [x] Layers: toggle Power, Resources, Pollution (core toggling + UI coloring)
- [x] Placement: select building type -> ghost preview -> confirm -> emit Command::PlaceBuilding

## Milestone 2 — UI Panels & Modals
- [x] Sidebar/outliner: buildings list, alerts (placeholder)
- [x] Building Detail modal: stats, production, actions (pause/demolish stubs)
- [x] Construction modal: building grid with costs; integrates with placement
- [x] Top resource bar; bottom bar with notifications and Advance Turn
- [x] Shortcuts: Space (advance turn), B (build), Esc (close top modal)
  - Note: Implemented Space/B/Esc in outpost-client

## Milestone 3 — Charts (egui_plot)
- [x] Power chart: generation vs consumption over recent turns
- [x] Resources chart: stock levels with production and consumption lines
- [x] Population chart: total plus employed/unemployed
- [x] On-demand updates: regenerate datasets when view opened or button clicked

## Milestone 4 — Persistence
- [x] Desktop save/load: RON or JSON files in user data directory; autosave every N turns
- [x] WASM save/load: initial implementation via browser LocalStorage with a simple profile selector (top-bar profile id field)
- [x] Versioning: include save format version and migration hook

## Milestone 5 — CI & Artifacts
 - [x] GitHub Actions matrix: windows-latest and ubuntu-latest
 - [x] Build outpost-core tests; build outpost-client in release
 - [x] WASM pipeline: wasm32-unknown-unknown with wasm-bindgen or trunk; publish dist bundle
 - [x] Upload native binaries and WASM bundle per commit/tag

## Milestone 6 — Assets & Polish
- [x] Add initial sprites for terrain, resources, and buildings
- [x] Performance: sprite batching, simple culling, reduce overdraw
- [x] Input normalization between desktop and WASM
- [x] QA smoke on Windows, Linux, and WASM

## Milestone 7 — New Desktop/WASM Tasks (2026-01-03)
- [x] Develop a way to check for the correct positioning of UI elements (inspection via properties/logging/events) and use it to verify that UI elements display where expected.
  - Implemented an egui-based debug overlay toggled with `F10` that displays the last-known screen rects for key UI panels (top bar, left sidebar, bottom bar). When enabled, the client emits `UiRectLogged` events each frame and logs them via `info!`. This provides both on-screen inspection and log auditing for UI placement verification (desktop and WASM). 2026-01-03 08:35.
- [x] Set the default desktop window resolution to 1920x1080.
  - On native builds, the primary `Window` now uses `WindowResolution::new(1920.0, 1080.0)`. WASM continues to use `fit_canvas_to_parent` targeting `#outpost-canvas`. 2026-01-03 08:35.
- [ ] Divide the program into at least two distinct scenes:
  1) Start Menu with buttons: New Game, Load Game, Settings, About, Exit
  2) Game Play with UI for playing the game
- [ ] Broad feature: more resources, materials, and goods.
  - [ ] minerals and elements
  - [ ] metals and alloys
  - [ ] food and drink
  - [ ] chemicals
  - [ ] energy and power
  - [ ] other goods
- [ ] Broad feature: banking and financial markets
- [ ] Broad feature: add more production chains of items
  - [ ] example: steel production 1 -- iron ore + carbon + arc furnace + oxygen --> steel
  - [ ] multiple ways to produce materials
- [ ] Broad feature: settlements launching satellites
  - [ ] buildings for launch infrastructure
  - [ ] components to manufacture satellites and launch vehicles
  - [ ] satellite and launch vehicle production chains
  - [ ] launch planning and operations
- [ ] Broad feature: exploring new planets with a special gateway
- [ ] Broad feature: more train mechanics — placing rails, defining routes, train composition, requests/supply
- [ ] Broad feature: a galaxy map showing relative positions of stars with gateways
- [ ] Broad feature: high‑level economy (supply/demand per settlement; trading/exchange)
- [ ] Broad feature: population migration mechanics and growth/decline
- [ ] Broad feature: population buildings — medical, education, security, entertainment, commerce
- [ ] Broad feature: underground excavation — underground settlements and mining
- [ ] Broad feature: terraforming and changing terrain

## Detailed Task Breakdown (Prompt-Ready)

### A. Core crate (outpost-core)
- [x] Create crates/outpost-core as a library crate
- [x] Implement GameState with minimal fields: turn, population, power, resources, buildings
- [x] Define Command: AdvanceTurn, PlaceBuilding { hex, building_type }, ToggleLayer { layer }
- [x] Define Event and apply(state, event)
- [x] Implement handle(state, command) -> Vec<Event)
- [x] Implement advance_turn(state) skeleton for production/consumption
- [x] Queries: get_power_snapshot, get_resource_series, get_population_snapshot
- [x] Serde derives and helpers for RON/JSON serialization
- [x] Unit tests: command handling and hex placement validation

### B. Client crate (outpost-client)
- [x] Create crates/outpost-client as a binary crate
- [x] Add dependencies: bevy, bevy_egui, egui_plot, serde, ron or serde_json
- [x] Bevy app setup: default plugins; window title "Outpost 3 (Desktop/WASM)"
- [x] Systems: camera pan/zoom, input mapping, hex renderer, selection, layer toggles
- [x] Egui UI: sidebar, modals, top/bottom bars, chart windows
- [x] Storage trait: load, save, list_profiles; desktop (fs) and wasm (IndexedDB) backends
- [x] Platform cfg guards for WASM (`target_arch = wasm32`)
- [x] Asset loading: fonts and placeholder sprites from assets directory
- [x] Smoke test: application boots and UI panel renders

### C. WASM support
 - [x] Add wasm32-unknown-unknown target to toolchain
 - [x] Provide minimal index.html loader (or Trunk configuration)
 - [x] Ensure Bevy uses web-friendly settings (canvas id, dpi, webgl2)
 - [x] Implement WASM storage backend using IndexedDB

### D. CI setup
- [x] Add .github/workflows/build.yml with matrix build and cargo cache
 - [x] Build artifacts: upload outpost-client binaries and WASM bundle

### E. Documentation
- [x] Update README with desktop and WASM run instructions
- [x] Document controls: pan (drag/WASD), zoom (wheel), Space/B/Esc shortcuts
- [x] Add architecture diagram: core <-> client, storage backends (see docs/diagrams/architecture.md)

## Run Tips (initial)
- Desktop: cargo run -p outpost-client (after crates are created)
- WASM with wasm-server-runner: add wasm32 target; set runner to wasm-server-runner; cargo run --target wasm32-unknown-unknown -p outpost-client
- WASM with trunk: trunk serve

## Decision Log
- 2026-01-01: Choose Bevy + bevy_egui, no webviews; charts via egui_plot; storage via files (desktop) and IndexedDB (WASM)
 - 2026-01-01: Implemented outpost-core scaffolding with queries, serialization helpers, and unit + property tests; all tests passing
 - 2026-01-01: Implemented camera pan/zoom with clamped limits in outpost-client; added unit and property-based tests (passing)
 - 2026-01-01 16:42: Implemented outpost-client Storage trait with desktop filesystem JSON backend and WASM stub; added unit and property-based tests; passing under `cargo test -p outpost-client`
 - 2026-01-01 18:44: Implemented 50x50 hex grid rendering, hover/click selection with highlight, Space shortcut to advance turn, and egui hover tooltip in outpost-client
 - 2026-01-02 08:32: Added Cargo features `desktop` (default) and `wasm` to outpost-client; feature-gated storage backends with `SelectedStorage` and `new_default_storage()`. Verified via cargo tests for both feature sets.
 - 2026-01-02 09:03: Implemented Power chart window in outpost-client using `egui_plot`, added rolling GameState history buffer and a small unit test for data preparation; verified via `cargo test -p outpost-client`.

 - 2026-01-02 09:12: Implemented Building Detail modal in outpost-client with stats display and Pause/Resume and Demolish stub actions; opens when selecting a placed building or after placement; state updates applied safely post-UI; verified via `cargo test -p outpost-client`.

 - 2026-01-02 09:21: Implemented Resources chart window in outpost-client using `egui_plot`, showing stock, derived production, and derived consumption lines from `GameState` history; added helper `prepare_resource_series` with unit test; UI toggle in top bar; verified via `cargo test -p outpost-client`.

 - 2026-01-02 10:58: Implemented Population chart window in outpost-client using `egui_plot`, plotting Total, Employed, and Unemployed from `GameState` history; added helper `prepare_population_series` with unit test and UI toggle in top bar.

 - 2026-01-02 11:27: Added minimal GitHub Actions CI workflow to build and test on Windows and Linux, including Bevy dependencies for Ubuntu; marks Milestone 0 CI item complete and D. CI setup first task complete.
 
  - 2026-01-02 11:31: Implemented Construction modal in outpost-client with a building grid (Solar/Wind/Hab) displaying costs, selection highlighting, and a details panel. Integrated with build placement: opening via B, hover to preview, left-click places, modal auto-closes on placement and opens Building Detail. Marks Milestone 2 Construction modal complete.
 
  - 2026-01-02 11:55: Implemented desktop persistence in outpost-client using JSON via `FsStorage` (user data directory). Added top-bar Save/Load with editable profile id and status text. Added autosave every 5 turns triggered on advancing turn (both Space shortcut and button). Marked Milestone 4 Desktop save/load complete. Tests for storage roundtrip already pass under `cargo test -p outpost-client`.
 
  - 2026-01-02 12:01: Implemented save format versioning with migration hook in `outpost-client` storage. Saves now wrap `GameState` in a versioned envelope (`version` + `kind`), with backward compatibility for legacy raw `GameState` JSON. Marks Milestone 4 Versioning complete.

 - 2026-01-02 12:58: Verified CI runs on a matrix of `windows-latest` and `ubuntu-latest` and marked Milestone 5 item 60 complete.

 - 2026-01-02 13:15: CI workflow runs `cargo test -p outpost-core` and `cargo build -p outpost-client --release`; marked Milestone 5 item 61 complete.

 - 2026-01-02 14:29: Implemented UI Layer toggles (Power/Resources/Pollution) in the top bar using core `ToggleLayer` command and added grid tinting to visualize active layers. This completes B. Client crate items for systems (including layer toggles) and egui UI panels/bars.

 - 2026-01-02 14:48: Added architecture diagram documenting core <-> client and storage backends. Diagram is available at `docs/diagrams/architecture.md`. Marked E. Documentation item complete.
 
 - 2026-01-02 14:53: Implemented client asset loading. On startup, the client attempts to apply `assets/fonts/Inter-Regular.ttf` as the primary egui font on desktop builds and queues placeholder sprites via Bevy `AssetServer` from `assets/sprites/terrain_placeholder.png` and `assets/sprites/buildings_placeholder.png`. Top bar indicates whether the custom font is active. Marks B. Client crate "Asset loading" item complete.

 - 2026-01-02 14:59: Implemented WASM save/load using browser LocalStorage to maintain a synchronous `Storage` trait (JSON envelope with versioning). The UI uses the existing top-bar profile id field for selecting save slots; autosave behavior mirrors desktop. This marks Milestone 4 "WASM save/load" complete. IndexedDB migration remains planned under C. WASM support.
 
 - 2026-01-02 15:03: Added Trunk-compatible `crates/outpost-client/index.html` with a dedicated canvas (`#outpost-canvas`) and configured Bevy window on `wasm32` to target this canvas and fit to parent. This completes C.97 (index.html/Trunk loader) and C.98 (web-friendly Bevy window settings).
 
 - 2026-01-02 15:08: Added GitHub Actions WASM job to build the `outpost-client` WebAssembly bundle using Trunk and upload the `crates/outpost-client/dist` folder as an artifact. This completes Milestone 5 item 62 (WASM pipeline) and D.103 (artifact uploads including WASM). Native binary artifacts remain as before. Tag-based release uploads are still pending (Milestone 5 item 63 remains open).
 
 - 2026-01-02 19:56: Implemented GitHub Releases uploads on tags for native Windows (`outpost-client.exe`) and Linux (`outpost-client`) binaries, and zipped WASM bundle built via Trunk. CI now triggers on tags and publishes artifacts using `softprops/action-gh-release`. This completes Milestone 5 item 63.

- 2026-01-02 21:07: Added `rust-toolchain.toml` declaring `wasm32-unknown-unknown` in `targets`, ensuring the WASM target is available locally and in CI by default. Marks C.96 complete.

- 2026-01-02 21:11: Implemented input normalization across desktop and WASM. Mouse drag deltas are adjusted by the window DPI scale factor (PrimaryWindow `scale_factor`) to equalize pan speed, and scroll wheel deltas normalize `Pixel` vs `Line` units (approx. 100 px = 1 line) for consistent zoom. Marks Milestone 6 "Input normalization between desktop and WASM" complete.

- 2026-01-02 21:17: Added initial sprite support in `outpost-client`. Each hex now renders a terrain placeholder sprite (as a child underlay) and placed buildings spawn a building placeholder sprite above the grid. Sprites are loaded from `assets/sprites/terrain_placeholder.png` and `assets/sprites/buildings_placeholder.png` via `AssetServer`, with the previous colored hex mesh retained for grid highlights and layer tints. This completes Milestone 6 item 66 (initial sprites).

- 2026-01-02 21:24: Implemented WASM storage backend using IndexedDB in `outpost-client`. The `Storage` trait remains synchronous by mirroring writes to `LocalStorage` immediately and performing IndexedDB writes asynchronously; loads prefer `LocalStorage` cache while refreshing from IndexedDB in the background. Updated `Cargo.toml` to include `indexed_db_futures`, `wasm-bindgen-futures`, and required `web-sys` features. `SelectedStorage` on WASM now uses the new IndexedDB backend. This completes item C.99.

- 2026-01-02 21:42: Implemented simple performance improvements in `outpost-client`: added a visibility culling system that hides hex tiles and building sprites outside the camera viewport (with adjustable world-margin) and made terrain underlay sprites opaque to reduce blending overdraw. Grid already uses a shared mesh and sprites share textures to enable batching. Added a top-bar toggle and margin control for debugging culling. This completes Milestone 6 item 67 (Performance: batching, culling, overdraw).

- 2026-01-02 21:50: Added QA smoke checks for desktop and WASM. For desktop (Windows/Linux), the client recognizes `OUTPOST_SMOKE_SECONDS` to auto-exit after the given seconds; CI runs the binary with a 2-second timer (Linux uses `xvfb-run`). For WASM, added `bin/wasm_smoke.mjs` that verifies Trunk-built `dist` contains `index.html` with `#outpost-canvas` and a non-trivially sized `.wasm` bundle; CI runs this after the Trunk build. This completes Milestone 6 item 69.

- 2026-01-03 08:35: Implemented UI positioning verification tooling: `UiRectLogged` events with `info!` logging and a toggleable (`F10`) egui overlay displaying rects for top bar, left sidebar, and bottom bar. Verified positions visually and via logs on desktop; works on WASM as well. Also set the default native desktop resolution to 1920x1080 while keeping WASM canvas fit. Marks Milestone 7 first two items complete.
