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

use axum::extract::State;
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
    let list: Vec<ColonizeTargetWire> = engine
        .state
        .system_state
        .node_map
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
    /// Per-cell surface-temperature band.
    pub temperature: String,
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
    /// Map radius in hex rings.
    pub radius: u32,
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
    /// Infrastructure type (`road`, `rail`, `pipeline`).
    pub infra_type: String,
    /// Cargo throughput per turn (before tech modifiers).
    pub throughput: f32,
    /// Construction cost (abstract resource units).
    pub cost: f32,
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
    let Some(pm) = engine.state.planet_map.as_ref() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "no planet map — start a new game first" })),
        )
            .into_response();
    };

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
                temperature: format!("{:?}", cell.temperature),
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
        })
        .collect();

    Json(PlanetMapWire {
        seed: pm.seed,
        radius: pm.radius,
        hexes,
        edges,
    })
    .into_response()
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
