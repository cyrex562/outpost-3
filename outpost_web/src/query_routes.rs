//! Read-only wizard-support REST endpoints (issue #220).
//!
//! `outpost_tauri/src/commands.rs` has bespoke Tauri commands
//! (`get_colonize_targets`, `list_buildings`, `list_supply_packages`,
//! `get_planet_map`, `get_system_bodies`) that read directly from
//! `engine.state` rather than going through `outpost_core::Query` — none of
//! those have a core `Query` variant, and the WS query dispatch (`ws.rs`)
//! only forwards a handful of the existing `Query` variants anyway. This
//! module mirrors those same read paths for `outpost_web`'s shared
//! `AppState.engine` (the same engine `ws.rs`'s `NewGame` flow bootstraps),
//! so browser-mode screens — the colony-founding wizard in particular — can
//! reach them without a Tauri IPC bridge.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use serde_json::json;

use outpost_core::system::BodyKind;

use crate::state::AppState;

/// Colonizable target for the founding wizard.
#[derive(Debug, Serialize)]
pub struct ColonizeTargetWire {
    /// Stable identifier of the target body.
    pub body_id: String,
    /// Display name of the target body.
    pub body_name: String,
    /// Body kind (`InnerPlanet`, `Moon`, `AsteroidBelt`, ...).
    pub kind: String,
    /// Orbital distance from the primary star, in AU.
    pub distance_au: f32,
    /// Body habitability score, `0..=100`.
    pub habitability: u8,
    /// Starlight reaching this body, where Sol at 1 AU is `1.0` (issue #413).
    ///
    /// In the founding wizard because it decides what solar power is worth
    /// here (issue #415) — a landing-site input, so it has to be part of the
    /// comparison made *before* founding, not discovered after.
    pub insolation: f32,
    /// How vigorously bulk water moves here, `0.0`–`1.0` (issue #440).
    ///
    /// Alongside `insolation` for the same reason: it decides what an ocean
    /// current plant is worth on this body, so it belongs in the comparison
    /// made *before* founding.
    pub ocean_circulation: f32,
    /// Whether founding on this body is currently allowed.
    pub can_found: bool,
}

/// `GET /api/colonize-targets` — bodies the wizard can offer for founding.
///
/// Mirrors `outpost_tauri::commands::get_colonize_targets` against the
/// shared engine instead of a Tauri-managed one.
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn get_colonize_targets(State(state): State<AppState>) -> impl IntoResponse {
    let engine = state.engine.lock().expect("engine lock");
    let harsh_world_unlocked = engine
        .state
        .unlocked_capabilities
        .contains(outpost_core::system::HARSH_WORLD_CAPABILITY_ID);
    let node_map = &engine.state.system_state.node_map;
    let list: Vec<ColonizeTargetWire> = node_map
        .bodies
        .values()
        .filter(|b| {
            matches!(
                b.kind,
                BodyKind::InnerPlanet | BodyKind::Moon | BodyKind::AsteroidBelt
            )
        })
        .map(|b| ColonizeTargetWire {
            body_id: b.id.0.to_string(),
            body_name: b.name.clone(),
            kind: format!("{:?}", b.kind),
            distance_au: b.distance_au,
            habitability: b.habitability(),
            insolation: node_map.insolation_for(&b.id).unwrap_or(0.0),
            ocean_circulation: node_map.ocean_circulation_for(&b.id).unwrap_or(0.0),
            can_found: b.meets_founding_threshold() || harsh_world_unlocked,
        })
        .collect();
    Json(list)
}

/// A building option in the founding wizard.
#[derive(Debug, Serialize)]
pub struct BuildingOptionWire {
    /// Content-pack key of the building type.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Flavor description.
    pub description: String,
    /// Building category (`Extraction`, `Processing`, ...).
    pub category: String,
    /// Build-slot cost.
    pub slot_cost: u32,
    /// Labor required per turn.
    pub labor_per_turn: u32,
    /// Construction time in sols.
    pub construction_turns: u32,
    /// Commodity id/quantity pairs consumed to build this.
    pub construction_cost: Vec<(String, f64)>,
    /// Tech id required before this building can be queued, if any.
    pub tech_prerequisite: Option<String>,
    /// Whether this building is part of the default landing kit (issue #317) —
    /// lets the founding wizard pre-select it as the recommended loadout.
    pub starter_kit: bool,
    /// Most instances one colony may have, or `None` for unlimited. Lets the
    /// build UI grey out an option the engine would reject rather than
    /// offering a button that always errors.
    pub max_instances: Option<u32>,
}

/// `GET /api/buildings` — every building type in the loaded content pack.
///
/// Mirrors `outpost_tauri::commands::list_buildings` against the shared
/// engine. Returns `404` if no content pack is loaded yet (before `NewGame`).
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn list_buildings(State(state): State<AppState>) -> impl IntoResponse {
    let engine = state.engine.lock().expect("engine lock");
    let Some(registry) = engine.state.registry.as_ref() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "no content registry loaded" })),
        )
            .into_response();
    };
    let mut out: Vec<BuildingOptionWire> = registry
        .buildings()
        .map(|b| BuildingOptionWire {
            id: b.id.clone(),
            name: b.name.clone(),
            description: b.description.clone(),
            category: format!("{:?}", b.category),
            slot_cost: b.slot_cost,
            labor_per_turn: b.labor_required,
            construction_turns: b.construction_turns,
            construction_cost: b
                .construction_cost
                .iter()
                .map(|i| (i.id.clone(), i.quantity))
                .collect(),
            tech_prerequisite: b.tech_prerequisite.clone(),
            starter_kit: b.starter_kit,
            max_instances: b.max_instances,
        })
        .collect();
    out.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| a.name.cmp(&b.name))
    });
    Json(out).into_response()
}

/// A starter supply package option surfaced in the founding wizard.
#[derive(Debug, Serialize)]
pub struct SupplyPackageWire {
    /// Content-pack key of this supply package.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Flavor description.
    pub description: String,
    /// Commodity id/per-100-colonist quantity pairs.
    pub commodities: Vec<(String, f64)>,
}

/// `GET /api/supply-packages` — every starter supply package in the loaded
/// content pack.
///
/// Mirrors `outpost_tauri::commands::list_supply_packages` against the
/// shared engine. Returns `404` if no content pack is loaded yet.
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn list_supply_packages(State(state): State<AppState>) -> impl IntoResponse {
    let engine = state.engine.lock().expect("engine lock");
    let Some(registry) = engine.state.registry.as_ref() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "no content registry loaded" })),
        )
            .into_response();
    };
    let mut out: Vec<SupplyPackageWire> = registry
        .supply_packages()
        .map(|p| SupplyPackageWire {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            commodities: p
                .commodities
                .iter()
                .map(|i| (i.id.clone(), i.quantity))
                .collect(),
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Json(out).into_response()
}

/// One hex on the planet surface, enriched with UI-relevant fields.
#[derive(Debug, Serialize)]
pub struct PlanetHexWire {
    /// Axial coordinate, q component.
    pub q: i32,
    /// Axial coordinate, r component.
    pub r: i32,
    /// Stable site identifier for this cell, if one has been minted.
    pub site_id: String,
    /// Surface terrain class.
    pub terrain: String,
    /// Surface biome class.
    pub biome: String,
    /// Normalised elevation in `[0.0, 1.0]`.
    pub elevation: f32,
    /// Geothermal gradient in `[0.0, 1.0]` (issue #412) — how shallow magma
    /// sits beneath this hex. Surfaced so it can inform a landing-site
    /// choice, which is exactly where it matters.
    pub geothermal_gradient: f32,
    /// Per-cell surface-temperature band.
    pub temperature: String,
    /// Fraction of this cell covered by water/ice, in `[0.0, 1.0]` (issue #316).
    pub water_coverage: f32,
    /// Vegetation density in this cell, in `[0.0, 1.0]` (issue #316).
    pub vegetation_density: f32,
    /// Contamination severity in `[0.0, 1.0]` from waste overflow (issue
    /// #387). `0.0` is pristine.
    pub contamination: f32,
    /// Resource deposits present in this cell.
    pub deposits: Vec<DepositWire>,
    /// Whether a colony could be founded on this cell.
    pub habitable: bool,
    /// Landing-site suitability score — higher is better.
    pub suitability: f32,
    /// Name of the colony occupying this cell, if any.
    pub occupied_by: Option<String>,
    /// Id of the colony occupying this cell, if any (persistent planet map,
    /// phase A1) — lets a map node link through to `/colony/:id`.
    pub occupant_colony_id: Option<String>,
}

/// A resource deposit within a [`PlanetHexWire`].
#[derive(Debug, Serialize)]
pub struct DepositWire {
    /// Content-pack commodity id.
    pub commodity_id: String,
    /// Relative richness in `(0.0, 1.0]`.
    pub richness: f32,
}

/// The full planet hex map, as returned to the founding wizard.
#[derive(Debug, Serialize)]
pub struct PlanetMapWire {
    /// Procedural-generation seed used for this planet.
    pub seed: u64,
    /// Column count of the map (wraps east-west).
    pub width: u32,
    /// Row count of the map (`r = 0` / `r = height - 1` are the poles).
    pub height: u32,
    /// Every cell on the map.
    pub hexes: Vec<PlanetHexWire>,
    /// Infrastructure edges connecting colony nodes (map/nav plan phase A3).
    pub edges: Vec<InfraEdgeWire>,
}

/// An infrastructure edge between two colonies, for planet-map rendering
/// (map/nav plan phase A3). Endpoints are resolved to hex positions on the
/// frontend via each colony's `occupant_colony_id`.
#[derive(Debug, Serialize)]
pub struct InfraEdgeWire {
    /// Id of the colony the edge runs from.
    pub from_colony_id: String,
    /// Id of the colony the edge runs to.
    pub to_colony_id: String,
    /// Infrastructure type (`road`, `rail`, `pipeline`, `powerline`).
    pub infra_type: String,
    /// Cargo (or, for a powerline, power) throughput per turn, before tech
    /// modifiers.
    pub throughput: f32,
    /// Construction cost (abstract resource units).
    pub cost: f32,
    /// Fraction of throughput lost in transit, in `[0.0, 1.0]` (issue #383).
    pub loss_pct: f32,
}

/// `GET /api/colony-screen/:id` — the full colony management screen bundle.
///
/// Mirrors the Tauri `colony_screen` query against the shared engine. Browser
/// mode previously had no way to fetch this at all, so every panel driven by it
/// — buildings, stockpile, colony resources — rendered empty there (issue #307
/// stage 4 needed the buildings panel to work in a browser to be testable).
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn get_colony_screen(
    State(state): State<AppState>,
    Path(colony_id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    let engine = state.engine.lock().expect("engine lock");
    match engine.query(&outpost_core::Query::ColonyScreen { colony_id }) {
        Ok(outpost_core::QueryResult::ColonyScreen(data)) => Json(data).into_response(),
        Ok(other) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("unexpected query result: {other:?}") })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("{e:?}") })),
        )
            .into_response(),
    }
}

/// `GET /api/planet-map` — the current planet's hex map with per-cell
/// metadata.
///
/// Mirrors `outpost_tauri::commands::get_planet_map` against the shared
/// engine. Returns `404` if no planet map has been seeded yet (before
/// `NewGame`).
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn get_planet_map(State(state): State<AppState>) -> impl IntoResponse {
    let engine = state.engine.lock().expect("engine lock");
    let Some(pm) = engine.state.home_map() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "no planet map — start a new game first" })),
        )
            .into_response();
    };
    Json(build_planet_map_wire(pm, &engine)).into_response()
}

/// `GET /api/balance-scalars` — every tunable balance dial and its current
/// value, for the live playtesting editor.
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn get_balance_scalars(State(state): State<AppState>) -> impl IntoResponse {
    let engine = state.engine.lock().expect("engine lock");
    match engine.query(&outpost_core::Query::BalanceScalars) {
        Ok(outpost_core::QueryResult::BalanceScalars(rows)) => Json(rows).into_response(),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "unexpected query result" })),
        )
            .into_response(),
    }
}

/// `GET /api/trade-routes` — every trade route in the planetary trade
/// network (issue #363), infrastructure-linked or manually added alike.
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn get_trade_routes(State(state): State<AppState>) -> impl IntoResponse {
    let engine = state.engine.lock().expect("engine lock");
    match engine.query(&outpost_core::Query::TradeRoutes) {
        Ok(outpost_core::QueryResult::TradeRoutes(routes)) => Json(routes).into_response(),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "unexpected query result" })),
        )
            .into_response(),
    }
}

/// `GET /api/body-surface/:id` — the surface of any system body, live if it has
/// been settled and a procedurally-generated preview if it has not.
///
/// Returning the stored surface in preference to a fresh preview is what makes
/// this usable as *the* surface view for every world (issue #300): a settled
/// body's colonies and infrastructure show up, while an unvisited one still
/// renders. The two agree cell-for-cell — a body's stored map is generated by
/// `body_surface_preview` on first settlement — so the switch is invisible
/// apart from the colonies appearing.
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn get_body_surface(
    State(state): State<AppState>,
    Path(body_id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    let engine = state.engine.lock().expect("engine lock");
    let id = outpost_core::system::BodyId(body_id);
    if let Some(pm) = engine.state.map_for_body(&id) {
        return Json(build_planet_map_wire(pm, &engine)).into_response();
    }
    match engine.body_surface_preview(&id) {
        Ok(pm) => Json(build_planet_map_wire(&pm, &engine)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Serialize a [`outpost_core::map::PlanetMap`] to the wire shape. Shared by
/// the live planet map and the per-body surface preview; colony/occupancy
/// fields resolve against the live colony list (empty for a fresh preview).
fn build_planet_map_wire(
    pm: &outpost_core::map::PlanetMap,
    engine: &outpost_core::GameEngine,
) -> PlanetMapWire {
    let coord_to_site: std::collections::HashMap<_, _> =
        pm.sites.iter().map(|(sid, coord)| (*coord, *sid)).collect();

    let colony_names: std::collections::HashMap<_, _> = engine
        .state
        .colonies
        .iter()
        .map(|c| (c.id, c.name.clone()))
        .collect();
    let coord_to_colony: std::collections::HashMap<_, _> = pm
        .colonies
        .iter()
        .filter_map(|node| {
            colony_names
                .get(&node.colony_id)
                .map(|name| (node.coord, (node.colony_id, name.clone())))
        })
        .collect();

    let mut hexes: Vec<PlanetHexWire> = pm
        .cells
        .values()
        .map(|cell| {
            let site_id = coord_to_site
                .get(&cell.coord)
                .map(|sid| sid.0.to_string())
                .unwrap_or_default();
            PlanetHexWire {
                q: cell.coord.q,
                r: cell.coord.r,
                site_id,
                terrain: format!("{:?}", cell.terrain),
                biome: format!("{:?}", cell.biome),
                elevation: cell.elevation,
                geothermal_gradient: cell.geothermal_gradient,
                temperature: format!("{:?}", cell.temperature),
                water_coverage: cell.water_coverage,
                vegetation_density: cell.vegetation_density,
                contamination: cell.contamination,
                deposits: cell
                    .deposits
                    .iter()
                    .map(|d| DepositWire {
                        commodity_id: d.commodity_id.clone(),
                        richness: d.richness,
                    })
                    .collect(),
                habitable: cell.is_habitable(),
                suitability: cell.suitability(),
                occupied_by: coord_to_colony
                    .get(&cell.coord)
                    .map(|(_, name)| name.clone()),
                occupant_colony_id: coord_to_colony
                    .get(&cell.coord)
                    .map(|(id, _)| id.to_string()),
            }
        })
        .collect();

    // Stable ordering so the frontend doesn't churn cell z-order between calls.
    hexes.sort_by_key(|h| (h.r, h.q));

    let edges: Vec<InfraEdgeWire> = pm
        .edges
        .iter()
        .map(|e| InfraEdgeWire {
            from_colony_id: e.from.to_string(),
            to_colony_id: e.to.to_string(),
            infra_type: format!("{:?}", e.infra_type).to_lowercase(),
            throughput: e.throughput,
            cost: e.cost,
            loss_pct: e.loss_pct,
        })
        .collect();

    PlanetMapWire {
        seed: pm.seed,
        width: pm.width,
        height: pm.height,
        hexes,
        edges,
    }
}

/// A body on the system map, positioned for rendering.
#[derive(Debug, Serialize)]
pub struct SystemBodyWire {
    /// Stable identifier of the body.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Body kind (`InnerPlanet`, `Moon`, `AsteroidBelt`, ...).
    pub kind: String,
    /// Orbital role (`Primary`, `Satellite`, ...).
    pub role: String,
    /// Orbital distance from the primary star, in AU.
    pub distance_au: f32,
    /// Whether this body is a valid colonization target.
    pub colonizable: bool,
    /// Atmospheric thickness/density band (issue #197).
    pub atmosphere_density: String,
    /// Atmospheric chemical hazard band (issue #197).
    pub atmosphere_hazard: String,
    /// Surface-temperature band.
    pub temperature: String,
    /// Surface gravity, in units of Earth gravity.
    pub gravity_g: f32,
    /// Radiation hazard band.
    pub radiation: String,
    /// Base habitability score, `0..=100`.
    pub habitability: u8,
    /// Base habitability multiplier applied to production.
    pub habitability_modifier: f32,
    /// Habitability score after tech-driven mitigations are applied (issue
    /// #185). Equal to `habitability` when no mitigation applies.
    pub habitability_effective: u8,
    /// Habitability modifier after tech-driven mitigations are applied
    /// (issue #185). Equal to `habitability_modifier` when no mitigation applies.
    pub habitability_modifier_effective: f32,
    /// Starlight reaching this body, in units where Sol at 1 AU is `1.0`
    /// (issue #413).
    ///
    /// A moon reports its parent's, since its own `distance_au` is measured
    /// from the planet. Surfaced because it is a landing-site input — it
    /// decides what solar power is worth here (issue #415) — so it has to be
    /// visible *before* founding, not after.
    pub insolation: f32,
    /// How vigorously bulk water moves here, `0.0`–`1.0` (issue #440).
    ///
    /// Derived rather than raw: `tidally_locked` and `rotation_period_hours`
    /// are on this payload too, but a moon's dominant term is tidal forcing
    /// from its parent, which needs a lookup the frontend cannot do.
    pub ocean_circulation: f32,
    /// Surface/composition archetype (issue #196) — flavor/authoring
    /// guidance, not a habitability input.
    pub subtype: String,
    /// Whether the body is tidally locked to its parent.
    pub tidally_locked: bool,
    /// Axial tilt, in degrees.
    pub axial_tilt_deg: f32,
    /// Rotation period, in hours.
    pub rotation_period_hours: f32,
    /// Number of natural satellites.
    pub moon_count: u32,
    /// Display name of the body this one orbits, if any.
    pub parent_body_name: Option<String>,
    /// Per-category production modifiers (issue #184) — category name to
    /// multiplier, e.g. `("power_output", 1.3)`. Empty when unauthored.
    pub category_modifiers: Vec<(String, f32)>,
    /// Density-zoned annulus profile for belt-kind bodies (system-screen fix
    /// B2). `None` for non-belt bodies.
    pub belt_profile: Option<BeltProfileWire>,
}

/// One angular zone of a belt's annulus (system-screen fix B2).
#[derive(Debug, Serialize)]
pub struct BeltZoneWire {
    /// Angular start of the zone, in degrees `[0, 360)`.
    pub start_deg: f32,
    /// Angular sweep of the zone, in degrees.
    pub sweep_deg: f32,
    /// Fill density `[0, 1]` — drives the annulus fill opacity.
    pub density: f32,
}

/// Radial/angular density profile of a belt, for annulus rendering
/// (system-screen fix B2).
#[derive(Debug, Serialize)]
pub struct BeltProfileWire {
    /// Inner radius of the annulus, in AU.
    pub inner_au: f32,
    /// Outer radius of the annulus, in AU.
    pub outer_au: f32,
    /// Angular zones subdividing the annulus.
    pub zones: Vec<BeltZoneWire>,
}

/// `GET /api/system-name` — the generated star system's display name.
///
/// Its own tiny route rather than a field on `/api/system-bodies` so that
/// endpoint's array shape stays unchanged. Empty for a system that predates
/// seed-derived naming; callers fall back to a generic label.
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn get_system_name(State(state): State<AppState>) -> impl IntoResponse {
    let engine = state.engine.lock().expect("engine lock");
    let name = engine.state.system_state.node_map.system_name.clone();
    Json(json!({ "name": name }))
}

/// `GET /api/system-bodies` — every body in the system, with rendering hints.
///
/// Mirrors `outpost_tauri::commands::get_system_bodies` against the shared
/// engine.
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn get_system_bodies(State(state): State<AppState>) -> impl IntoResponse {
    let engine = state.engine.lock().expect("engine lock");
    let node_bodies = &engine.state.system_state.node_map.bodies;
    let bodies: Vec<SystemBodyWire> = node_bodies
        .values()
        .map(|b| SystemBodyWire {
            id: b.id.0.to_string(),
            name: b.name.clone(),
            kind: format!("{:?}", b.kind),
            role: format!("{:?}", b.role),
            distance_au: b.distance_au,
            insolation: engine
                .state
                .system_state
                .node_map
                .insolation_for(&b.id)
                .unwrap_or(0.0),
            ocean_circulation: engine
                .state
                .system_state
                .node_map
                .ocean_circulation_for(&b.id)
                .unwrap_or(0.0),
            colonizable: matches!(
                b.kind,
                BodyKind::InnerPlanet | BodyKind::Moon | BodyKind::AsteroidBelt
            ),
            atmosphere_density: format!("{:?}", b.atmosphere_density),
            atmosphere_hazard: format!("{:?}", b.atmosphere_hazard),
            temperature: format!("{:?}", b.temperature),
            gravity_g: b.gravity_g,
            radiation: format!("{:?}", b.radiation),
            habitability: b.habitability(),
            habitability_modifier: b.habitability_modifier(),
            habitability_effective: b
                .habitability_with_mitigations(&engine.state.habitability_mitigations),
            habitability_modifier_effective: b
                .habitability_modifier_with_mitigations(&engine.state.habitability_mitigations),
            subtype: format!("{:?}", b.subtype),
            tidally_locked: b.tidally_locked,
            axial_tilt_deg: b.axial_tilt_deg,
            rotation_period_hours: b.rotation_period_hours,
            moon_count: b.moon_count,
            parent_body_name: b
                .parent_body
                .as_ref()
                .and_then(|pid| node_bodies.get(pid))
                .map(|p| p.name.clone()),
            category_modifiers: b
                .modifiers
                .iter()
                .map(|m| (format!("{:?}", m.category), m.multiplier))
                .collect(),
            belt_profile: b.belt_profile.as_ref().map(|p| BeltProfileWire {
                inner_au: p.inner_au,
                outer_au: p.outer_au,
                zones: p
                    .zones
                    .iter()
                    .map(|z| BeltZoneWire {
                        start_deg: z.start_deg,
                        sweep_deg: z.sweep_deg,
                        density: z.density,
                    })
                    .collect(),
            }),
        })
        .collect();
    Json(bodies)
}

/// An established outpost, for the outposts management view (issue #243).
#[derive(Debug, Serialize)]
pub struct OutpostWire {
    /// Outpost UUID.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Colony that owns this outpost.
    pub parent_colony_id: String,
    /// System body it's anchored to.
    pub body_id: String,
    /// Display name of the anchor body.
    pub body_name: String,
    /// Total build-slot capacity.
    pub slot_capacity: u32,
    /// Build slots currently in use.
    pub slots_used: u32,
    /// Completed building types.
    pub buildings: Vec<String>,
    /// Pooled commodity stockpile as `(commodity_id, amount)` pairs.
    pub pool: Vec<(String, f64)>,
}

/// `GET /api/outposts` — every established outpost across all colonies.
///
/// Mirrors `outpost_tauri::commands::list_outposts` against the shared
/// engine. The frontend filters by `parent_colony_id` client-side.
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn list_outposts(State(state): State<AppState>) -> impl IntoResponse {
    let engine = state.engine.lock().expect("engine lock");
    let list: Vec<OutpostWire> = engine
        .state
        .outposts
        .iter()
        .map(|o| {
            let body_name = engine
                .state
                .system_state
                .node_map
                .bodies
                .get(&o.body_id)
                .map_or_else(|| "Unknown".to_string(), |b| b.name.clone());
            OutpostWire {
                id: o.id.to_string(),
                name: o.name.clone(),
                parent_colony_id: o.parent_colony_id.to_string(),
                body_id: o.body_id.0.to_string(),
                body_name,
                slot_capacity: o.slot_capacity,
                slots_used: o.slots_used(),
                buildings: o
                    .buildings
                    .iter()
                    .map(|b| b.building_type.clone())
                    .collect(),
                pool: o
                    .pool
                    .commodity_ids()
                    .map(|cid| (cid.to_string(), o.pool.amount(cid)))
                    .collect(),
            }
        })
        .collect();
    Json(list)
}

/// A body evaluated as a possible `EstablishOutpost` target for a given
/// parent colony (issue #241/#243).
#[derive(Debug, Serialize)]
pub struct OutpostTargetWire {
    /// Candidate body UUID.
    pub body_id: String,
    /// Display name.
    pub body_name: String,
    /// Body kind (`InnerPlanet`, `Moon`, `AsteroidBelt`, ...).
    pub kind: String,
    /// Orbital distance from the primary star, in AU.
    pub distance_au: f32,
    /// Distance from the parent colony's home body, in AU. `None` when the
    /// parent colony has no `home_body_id` (range gating is inert for it).
    pub distance_from_home_au: Option<f32>,
    /// Whether `EstablishOutpost` would currently accept this body.
    pub in_range: bool,
}

/// `GET /api/outpost-targets/:colony_id` — bodies a colony could establish
/// an outpost on, annotated with range-gate status (issue #241).
///
/// Mirrors `outpost_tauri::commands::get_outpost_targets` against the
/// shared engine.
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn get_outpost_targets(
    State(state): State<AppState>,
    axum::extract::Path(colony_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let engine = state.engine.lock().expect("engine lock");
    let Ok(cid) = outpost_core::colony::ColonyId::parse_str(&colony_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("invalid colony_id: {colony_id}") })),
        )
            .into_response();
    };
    let home_body = engine
        .state
        .colonies
        .iter()
        .find(|c| c.id == cid)
        .and_then(|c| c.home_body_id.as_ref())
        .and_then(|bid| engine.state.system_state.node_map.bodies.get(bid));
    let max_range_au = home_body.map(|_| {
        outpost_core::outpost::max_outpost_range_au(
            engine.state.system_state.node_map.propulsion_level,
            engine.state.outpost_range_bonus_au,
        )
    });
    let list: Vec<OutpostTargetWire> = engine
        .state
        .system_state
        .node_map
        .bodies
        .values()
        .map(|b| {
            let distance_from_home_au = home_body.map(|h| (h.distance_au - b.distance_au).abs());
            let in_range = match (distance_from_home_au, max_range_au) {
                (Some(d), Some(max)) => d <= max,
                _ => true,
            };
            OutpostTargetWire {
                body_id: b.id.0.to_string(),
                body_name: b.name.clone(),
                kind: format!("{:?}", b.kind),
                distance_au: b.distance_au,
                distance_from_home_au,
                in_range,
            }
        })
        .collect();
    Json(list).into_response()
}

/// Tech definition + player state (researched / `in_progress` / queued / available / locked).
#[derive(Debug, Serialize)]
pub struct TechNodeWire {
    /// Content-pack tech identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Content-authored category slug.
    pub category: String,
    /// Flavor description.
    pub description: String,
    /// DAG depth tier.
    pub tier: u32,
    /// Research-point cost.
    pub cost: f32,
    /// Prerequisite tech ids.
    pub prerequisites: Vec<String>,
    /// `researched | in_progress | queued | available | locked`.
    pub state: String,
    /// Fraction of `cost` completed, only meaningful when `state == "in_progress"`.
    pub progress: f32,
    /// Authored effects this tech grants on completion.
    pub effects: Vec<outpost_core::tech::TechEffect>,
    /// Zero-based position in `TechState.research_queue` (the actual FIFO
    /// drain order), only meaningful when `state == "queued"`. `None`
    /// otherwise, so the UI doesn't have to guess a queued tech's real
    /// position from an unrelated tier/category/name sort (issue #250 review).
    pub queue_position: Option<usize>,
}

/// `GET /api/tech-tree` — the full tech tree with per-node player state.
///
/// Mirrors `outpost_tauri::commands::get_tech_tree` against the shared
/// engine.
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn get_tech_tree(State(state): State<AppState>) -> impl IntoResponse {
    let engine = state.engine.lock().expect("engine lock");
    let Some(tech_registry) = engine.state.tech_registry.as_ref() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "no tech registry loaded" })),
        )
            .into_response();
    };

    let tech_state = &engine.state.tech_state;
    let current = tech_state.current_project.as_deref();

    let mut nodes: Vec<TechNodeWire> = tech_registry
        .all()
        .map(|tech| {
            let is_done = tech_state.is_researched(&tech.id);
            let is_active = current == Some(tech.id.as_str());
            let queue_position = tech_state.research_queue.iter().position(|t| t == &tech.id);
            let is_queued = queue_position.is_some();
            let prereqs_met = tech_state.prerequisites_met(tech);

            let state = if is_done {
                "researched"
            } else if is_active {
                "in_progress"
            } else if is_queued {
                "queued"
            } else if prereqs_met {
                "available"
            } else {
                "locked"
            };
            let progress = if is_active {
                tech_state.progress / tech.research_cost.max(0.001)
            } else {
                0.0
            };
            TechNodeWire {
                id: tech.id.clone(),
                name: tech.display_name.clone(),
                category: tech.category.clone(),
                description: tech.description.clone(),
                tier: tech.tier,
                cost: tech.research_cost,
                prerequisites: tech.prerequisites.clone(),
                state: state.to_owned(),
                progress,
                effects: tech.effects.clone(),
                queue_position,
            }
        })
        .collect();
    nodes.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then_with(|| a.category.cmp(&b.category))
            .then_with(|| a.name.cmp(&b.name))
    });
    Json(nodes).into_response()
}

/// One accumulated interrupt in the fast-forward digest.
#[derive(Debug, Serialize)]
pub struct DigestItemWire {
    /// Severity tier slug (`ambient` / `notable` / `urgent` / `blocking`).
    pub tier: String,
    /// Human-readable description.
    pub message: String,
    /// Colony this interrupt belongs to, if colony-scoped.
    pub colony_id: Option<String>,
    /// Whether the player has dismissed it.
    pub acknowledged: bool,
}

/// The return-from-fast-forward triage payload.
#[derive(Debug, Serialize)]
pub struct InterruptDigestWire {
    /// Sol the run stopped on.
    pub stopped_at_sol: u64,
    /// How many sols the run was asked to advance.
    pub sols_requested: u32,
    /// Message of the interrupt that halted the run, if one did.
    pub halting_message: Option<String>,
    /// Tier slug of the halting interrupt, if one halted the run.
    pub halting_tier: Option<String>,
    /// Below-threshold interrupts accumulated during the run.
    pub items: Vec<DigestItemWire>,
}

/// Render an interrupt tier as the same slug `ClientCommand::FastForward`
/// accepts, so the UI can round-trip a tier it read back into a threshold.
fn tier_slug(tier: outpost_core::interrupt::Tier) -> &'static str {
    use outpost_core::interrupt::Tier;
    match tier {
        Tier::Ambient => "ambient",
        Tier::Notable => "notable",
        Tier::Urgent => "urgent",
        Tier::Blocking => "blocking",
    }
}

/// `GET /api/interrupt-digest` — what happened during the last fast-forward
/// (issue #332).
///
/// The counterpart to `ClientCommand::FastForward`: the command reports only
/// that the run ended and why, and this is where the accumulated
/// below-threshold interrupts are read for the digest panel. Returns an empty
/// digest before any fast-forward has run.
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
pub async fn get_interrupt_digest(State(state): State<AppState>) -> impl IntoResponse {
    use outpost_core::{Query, QueryResult};

    let engine = state.engine.lock().expect("engine lock");
    let Ok(QueryResult::InterruptDigest(digest)) = engine.query(&Query::InterruptDigest) else {
        // `Query::InterruptDigest` is infallible in core and always returns this
        // variant, so this arm is unreachable in practice — reported as a server
        // error rather than unwrapped, per the no-panics rule.
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "interrupt digest query failed" })),
        )
            .into_response();
    };

    Json(InterruptDigestWire {
        stopped_at_sol: digest.stopped_at_turn,
        sols_requested: digest.turns_advanced,
        halting_message: digest.halting_interrupt.as_ref().map(|i| i.message.clone()),
        halting_tier: digest
            .halting_interrupt
            .as_ref()
            .map(|i| tier_slug(i.tier).to_owned()),
        items: digest
            .digest_items
            .iter()
            .map(|item| DigestItemWire {
                tier: tier_slug(item.interrupt.tier).to_owned(),
                message: item.interrupt.message.clone(),
                colony_id: item.interrupt.colony_id.map(|id| id.to_string()),
                acknowledged: item.acknowledged,
            })
            .collect(),
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::RuntimeConfig, routes::build_router, state::new_state};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_router() -> axum::Router {
        let state = new_state(RuntimeConfig::default());
        build_router(state)
    }

    #[tokio::test]
    async fn interrupt_digest_is_empty_before_any_fast_forward() {
        let router = test_router();
        let response = router
            .oneshot(
                Request::get("/api/interrupt-digest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["items"].as_array().unwrap().len(), 0);
        assert!(json["halting_message"].is_null());
    }

    /// The digest is only useful if a fast-forward actually populates it — an
    /// endpoint that always returns empty would pass the test above forever.
    #[tokio::test]
    async fn interrupt_digest_reports_a_halt_after_a_fast_forward() {
        use outpost_core::interrupt::Tier;
        use outpost_core::{Command, Event};

        let state = new_state(RuntimeConfig::default());
        {
            let mut engine = state.engine.lock().expect("engine lock");
            let founded = engine
                .apply(&Command::FoundColony {
                    name: "Digest".into(),
                    starting_population: 100,
                })
                .unwrap();
            let Event::ColonyFounded { colony_id, .. } = &founded[0] else {
                panic!("FoundColony must report the new colony")
            };
            let colony_id = *colony_id;

            // Stage a declining stability trajectory so the predictive warning
            // fires deterministically on the first sol.
            let tracker = engine
                .state
                .stability_trackers
                .entry(colony_id)
                .or_default();
            for s in [1.0f32, 0.7, 0.5, 0.3, 0.22] {
                tracker.push(s);
            }
            engine.state.populations[0].stability = 0.22;

            engine
                .apply(&Command::FastForward {
                    max_sols: 50,
                    threshold: Tier::Urgent,
                })
                .unwrap();
        }

        let response = build_router(state)
            .oneshot(
                Request::get("/api/interrupt-digest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            json["halting_tier"], "urgent",
            "the halting interrupt should be reported with its tier: {json}"
        );
        assert!(
            json["halting_message"]
                .as_str()
                .is_some_and(|m| !m.is_empty()),
            "the halt should carry a message: {json}"
        );
    }

    #[tokio::test]
    async fn buildings_404_before_content_loaded() {
        let router = test_router();
        let response = router
            .oneshot(Request::get("/api/buildings").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn supply_packages_404_before_content_loaded() {
        let router = test_router();
        let response = router
            .oneshot(
                Request::get("/api/supply-packages")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn planet_map_404_before_seeded() {
        let router = test_router();
        let response = router
            .oneshot(Request::get("/api/planet-map").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn tech_tree_404_before_content_loaded() {
        let router = test_router();
        let response = router
            .oneshot(Request::get("/api/tech-tree").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn colonize_targets_empty_list_on_fresh_engine() {
        let router = test_router();
        let response = router
            .oneshot(
                Request::get("/api/colonize-targets")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn system_bodies_empty_list_on_fresh_engine() {
        let router = test_router();
        let response = router
            .oneshot(
                Request::get("/api/system-bodies")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn outposts_empty_list_on_fresh_engine() {
        let router = test_router();
        let response = router
            .oneshot(Request::get("/api/outposts").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    /// Issue #363: `/api/trade-routes` exposes `Query::TradeRoutes` — browser
    /// mode's read side of the trade-route UI.
    #[tokio::test]
    async fn trade_routes_empty_list_on_fresh_engine() {
        let router = test_router();
        let response = router
            .oneshot(
                Request::get("/api/trade-routes")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn outpost_targets_rejects_invalid_colony_id() {
        let router = test_router();
        let response = router
            .oneshot(
                Request::get("/api/outpost-targets/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn outpost_targets_empty_list_on_fresh_engine() {
        let router = test_router();
        let bogus = uuid::Uuid::new_v4();
        let response = router
            .oneshot(
                Request::get(format!("/api/outpost-targets/{bogus}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 0);
    }
}
