//! Admin and star-map handlers.

use actix_web::{web, HttpResponse, Result};
use serde_json::json;
use uuid::Uuid;

use outpost_core::content::{
    BuildingCategory, BuildingDefinition, ConstructionCost, EventCategory, EventSeverity,
    PowerRequirement, ResourceCategory, ResourceDefinition, ResourcePhase, StorageRequirements,
};
use outpost_core::domain::{
    Atmosphere, BodyType, CelestialBody, CelestialBodyId, Recipe, Site, SiteId, SiteType,
    StarSystem, StarSystemId, Temperature,
};
use crate::db::SharedRepository;
use crate::services::SimulationService;
use std::collections::HashMap;

// ─── Form types ──────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct CreateSystemForm {
    pub name: String,
}

#[derive(serde::Deserialize)]
pub struct CreateBodyForm {
    pub system_id: String,
    pub name: String,
    pub body_type: String,
}

#[derive(serde::Deserialize)]
pub struct EditBodyForm {
    pub name: String,
    pub body_type: String,
    pub gravity: Option<f64>,
    pub radius_km: Option<f64>,
}

#[derive(serde::Deserialize)]
pub struct CreateSiteForm {
    pub system_id: String,
    pub body_id: String,
    pub name: String,
    pub site_type: String,
    pub population: u32,
}

#[derive(serde::Deserialize)]
pub struct EditSiteForm {
    pub name: String,
    pub site_type: String,
    pub population: u64,
}

#[derive(serde::Deserialize)]
pub struct CreateEventForm {
    pub site_id: String,
    pub event_id: String,
    pub title: String,
    pub severity: String,
}

#[derive(serde::Deserialize)]
pub struct ResourceDefinitionForm {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub base_value: f64,
    pub tradeable: Option<String>,
    pub consumable: Option<String>,
    pub phase: String,
    pub density_kg_per_m3: Option<f64>,
}

#[derive(serde::Deserialize)]
pub struct BuildingDefinitionForm {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub worker_capacity: u32,
    pub power_base_mw: Option<f64>,
    pub power_output_mw: Option<f64>,
    pub morale_bonus: Option<f64>,
    pub upgradeable: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct RecipeForm {
    pub id: String,
    pub name: String,
    pub building_types: String,
    pub inputs_json: String,
    pub outputs_json: String,
    pub processing_time_ticks: u64,
    pub required_deposit: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct SystemBodyEditForm {
    pub name: String,
    pub body_type: String,
    pub gravity: Option<f64>,
    pub radius_km: Option<f64>,
}

#[derive(serde::Deserialize)]
pub struct SystemBodyCreateForm {
    pub name: String,
    pub body_type: String,
}

#[derive(serde::Deserialize)]
pub struct SystemSiteCreateForm {
    pub name: String,
    pub site_type: String,
    pub population: u32,
    pub body_id: String,
}

#[derive(serde::Deserialize)]
pub struct SystemSiteEditForm {
    pub name: String,
    pub site_type: String,
    pub population: u32,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn redirect(url: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .append_header(("Location", url))
        .finish()
}

fn parse_body_type(s: &str) -> BodyType {
    match s {
        "GasGiant" => BodyType::GasGiant,
        "Moon" => BodyType::Moon,
        "Asteroid" => BodyType::Asteroid,
        "IceGiant" => BodyType::IceGiant,
        "OrbitalStation" => BodyType::OrbitalStation,
        "Comet" => BodyType::Comet,
        "Dwarf" => BodyType::Dwarf,
        _ => BodyType::TerrestrialPlanet,
    }
}

fn parse_severity(s: &str) -> EventSeverity {
    match s {
        "warning" => EventSeverity::Warning,
        "critical" => EventSeverity::Critical,
        _ => EventSeverity::Info,
    }
}

fn parse_resource_category(s: &str) -> ResourceCategory {
    match s {
        "metallic_ore" => ResourceCategory::MetallicOre,
        "non_metallic_mineral" => ResourceCategory::NonMetallicMineral,
        "hydrocarbon" => ResourceCategory::Hydrocarbon,
        "atmospheric_gas" => ResourceCategory::AtmosphericGas,
        "hydrospheric" => ResourceCategory::Hydrospheric,
        "agricultural" => ResourceCategory::Agricultural,
        "forestry" => ResourceCategory::Forestry,
        "aquaculture" => ResourceCategory::Aquaculture,
        "biological" => ResourceCategory::Biological,
        "metal" => ResourceCategory::Metal,
        "non_metallic_material" => ResourceCategory::NonMetallicMaterial,
        "chemical" => ResourceCategory::Chemical,
        "biological_product" => ResourceCategory::BiologicalProduct,
        "energy_storage" => ResourceCategory::EnergyStorage,
        "life_support" => ResourceCategory::LifeSupport,
        "component" => ResourceCategory::Component,
        "electronics" => ResourceCategory::Electronics,
        "mechanical_parts" => ResourceCategory::MechanicalParts,
        "consumer_goods" => ResourceCategory::ConsumerGoods,
        "industrial_equipment" => ResourceCategory::IndustrialEquipment,
        "vehicle" => ResourceCategory::Vehicle,
        "currency" => ResourceCategory::Currency,
        "data" => ResourceCategory::Data,
        _ => ResourceCategory::Component,
    }
}

fn parse_building_category(s: &str) -> BuildingCategory {
    match s {
        "power_generation" => BuildingCategory::PowerGeneration,
        "mining" => BuildingCategory::Mining,
        "refining" => BuildingCategory::Refining,
        "manufacturing" => BuildingCategory::Manufacturing,
        "farming" => BuildingCategory::Farming,
        "storage" => BuildingCategory::Storage,
        "housing" => BuildingCategory::Housing,
        "services" => BuildingCategory::Services,
        "entertainment" => BuildingCategory::Entertainment,
        "education" => BuildingCategory::Education,
        "healthcare" => BuildingCategory::Healthcare,
        "infrastructure" => BuildingCategory::Infrastructure,
        _ => BuildingCategory::Infrastructure,
    }
}

fn parse_resource_phase(s: &str) -> ResourcePhase {
    match s {
        "liquid" => ResourcePhase::Liquid,
        "gas" => ResourcePhase::Gas,
        "plasma" => ResourcePhase::Plasma,
        "virtual" => ResourcePhase::Virtual,
        _ => ResourcePhase::Solid,
    }
}

// ─── Star map ────────────────────────────────────────────────────────────────

/// GET /map
pub async fn view_starmap(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let game_state = simulation.get_state().await;

    let mut systems_data: Vec<_> = game_state.galaxy.systems.values()
        .map(|s| {
            json!({
                "id": s.id.to_string(),
                "name": &s.name,
                "explored": s.explored,
                "body_count": s.bodies.len(),
                "site_count": s.sites.len(),
                "total_population": s.total_population(),
            })
        })
        .collect();
    systems_data.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    let mut context = tera::Context::new();
    context.insert("systems", &systems_data);

    let html = tmpl.render("map.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// ─── Admin index ─────────────────────────────────────────────────────────────

/// GET /admin
pub async fn admin_index(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let game_state = simulation.get_state().await;

    let total_systems = game_state.galaxy.systems.len();
    let total_bodies: usize = game_state.galaxy.systems.values().map(|s| s.bodies.len()).sum();
    let total_sites: usize = game_state.galaxy.systems.values().map(|s| s.sites.len()).sum();
    let total_buildings: usize = game_state.galaxy.systems.values()
        .flat_map(|s| s.sites.values())
        .map(|site| site.buildings.len())
        .sum();
    let total_resource_defs = game_state.resource_definitions.len();
    let total_building_defs = game_state.building_definitions.len();
    let total_events = game_state.event_log.len();

    let mut context = tera::Context::new();
    context.insert("total_systems", &total_systems);
    context.insert("total_bodies", &total_bodies);
    context.insert("total_sites", &total_sites);
    context.insert("total_buildings", &total_buildings);
    context.insert("total_resource_defs", &total_resource_defs);
    context.insert("total_building_defs", &total_building_defs);
    context.insert("total_events", &total_events);

    let html = tmpl.render("admin/index.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// ─── Admin: systems ───────────────────────────────────────────────────────────

/// GET /admin/systems
pub async fn admin_systems_list(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let game_state = simulation.get_state().await;

    let mut systems_data: Vec<_> = game_state.galaxy.systems.values()
        .map(|s| json!({
            "id": s.id.to_string(),
            "name": &s.name,
            "seed": s.seed,
            "explored": s.explored,
            "body_count": s.bodies.len(),
            "site_count": s.sites.len(),
        }))
        .collect();
    systems_data.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    let mut context = tera::Context::new();
    context.insert("systems", &systems_data);

    let html = tmpl.render("admin/systems.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// POST /admin/systems/create
pub async fn admin_systems_create(
    simulation: web::Data<SimulationService>,
    form: web::Form<CreateSystemForm>,
) -> Result<HttpResponse> {
    simulation.with_state_mut(|state| {
        let galaxy_id = state.galaxy.id;
        let new_system = StarSystem::new(galaxy_id, form.name.clone(), 42);
        state.galaxy.systems.insert(new_system.id, new_system);
    }).await;
    Ok(redirect("/admin/systems"))
}

/// POST /admin/systems/{id}/delete
pub async fn admin_systems_delete(
    simulation: web::Data<SimulationService>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let id_str = path.into_inner();
    if let Ok(uuid) = Uuid::parse_str(&id_str) {
        let system_id = StarSystemId(uuid);
        simulation.with_state_mut(|state| {
            state.galaxy.systems.remove(&system_id);
        }).await;
    }
    Ok(redirect("/admin/systems"))
}

/// POST /admin/systems/{id}/toggle-explored
pub async fn admin_systems_toggle_explored(
    simulation: web::Data<SimulationService>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let id_str = path.into_inner();
    if let Ok(uuid) = Uuid::parse_str(&id_str) {
        let system_id = StarSystemId(uuid);
        simulation.with_state_mut(|state| {
            if let Some(system) = state.galaxy.systems.get_mut(&system_id) {
                system.explored = !system.explored;
            }
        }).await;
    }
    Ok(redirect("/admin/systems"))
}

// ─── Admin: bodies ────────────────────────────────────────────────────────────

/// GET /admin/bodies
pub async fn admin_bodies_list(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let game_state = simulation.get_state().await;

    let mut bodies_data: Vec<_> = game_state.galaxy.systems.values()
        .flat_map(|s| {
            s.bodies.values().map(move |b| json!({
                "id": b.id.to_string(),
                "name": &b.name,
                "system_id": s.id.to_string(),
                "system_name": &s.name,
                "body_type": format!("{:?}", b.body_type),
                "gravity": b.gravity,
                "radius_km": b.radius_km,
                "sites_count": s.site_count_on_body(&b.id),
            }))
        })
        .collect();
    bodies_data.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    let systems_data: Vec<_> = game_state.galaxy.systems.values()
        .map(|s| json!({"id": s.id.to_string(), "name": &s.name}))
        .collect();

    let mut context = tera::Context::new();
    context.insert("bodies", &bodies_data);
    context.insert("systems", &systems_data);

    let html = tmpl.render("admin/bodies.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// POST /admin/bodies/create
pub async fn admin_bodies_create(
    simulation: web::Data<SimulationService>,
    form: web::Form<CreateBodyForm>,
) -> Result<HttpResponse> {
    let sys_uuid = Uuid::parse_str(&form.system_id)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid system ID"))?;
    let system_id = StarSystemId(sys_uuid);
    let body_type = parse_body_type(&form.body_type);

    simulation.with_state_mut(|state| {
        if let Some(system) = state.galaxy.systems.get_mut(&system_id) {
            let body = CelestialBody::new(
                system.id,
                form.name.clone(),
                body_type,
                Atmosphere::None,
                Temperature::Cold,
                0.5,
                3000.0,
                0.3,
            );
            system.add_body(body);
        }
    }).await;
    Ok(redirect("/admin/bodies"))
}

/// POST /admin/bodies/{id}/delete
pub async fn admin_bodies_delete(
    simulation: web::Data<SimulationService>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let id_str = path.into_inner();
    if let Ok(uuid) = Uuid::parse_str(&id_str) {
        let body_id = CelestialBodyId(uuid);
        simulation.with_state_mut(|state| {
            for system in state.galaxy.systems.values_mut() {
                if system.bodies.contains_key(&body_id) {
                    system.remove_body(&body_id);
                    break;
                }
            }
        }).await;
    }
    Ok(redirect("/admin/bodies"))
}

/// POST /admin/bodies/{id}/edit
pub async fn admin_bodies_edit(
    simulation: web::Data<SimulationService>,
    path: web::Path<String>,
    form: web::Form<EditBodyForm>,
) -> Result<HttpResponse> {
    let id_str = path.into_inner();
    if let Ok(uuid) = Uuid::parse_str(&id_str) {
        let body_id = CelestialBodyId(uuid);
        let body_type = parse_body_type(&form.body_type);
        let new_name = form.name.clone();
        let new_gravity = form.gravity;
        let new_radius = form.radius_km;
        simulation.with_state_mut(move |state| {
            for system in state.galaxy.systems.values_mut() {
                if let Some(body) = system.bodies.get_mut(&body_id) {
                    body.name = new_name.clone();
                    body.body_type = body_type;
                    if let Some(g) = new_gravity { body.gravity = g; }
                    if let Some(r) = new_radius { body.radius_km = r; }
                    break;
                }
            }
        }).await;
    }
    Ok(redirect("/admin/bodies"))
}

// ─── Admin: sites ─────────────────────────────────────────────────────────────

/// GET /admin/sites
pub async fn admin_sites_list(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let game_state = simulation.get_state().await;

    let mut sites_data: Vec<_> = game_state.galaxy.systems.values()
        .flat_map(|s| {
            s.sites.values().map(move |site| {
                let body_name = s.bodies.get(&site.body_id)
                    .map(|b| b.name.as_str())
                    .unwrap_or("Unknown");
                json!({
                    "id": site.id.to_string(),
                    "name": &site.name,
                    "site_type": format!("{:?}", site.site_type),
                    "body_id": site.body_id.to_string(),
                    "body_name": body_name,
                    "system_name": &s.name,
                    "population": site.population.total,
                    "building_count": site.buildings.len(),
                })
            })
        })
        .collect();
    sites_data.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    // Build system/body options for the create form
    let mut systems_with_bodies: Vec<_> = game_state.galaxy.systems.values()
        .map(|s| {
            let bodies: Vec<_> = s.bodies.values()
                .map(|b| json!({"id": b.id.to_string(), "name": &b.name}))
                .collect();
            json!({"id": s.id.to_string(), "name": &s.name, "bodies": bodies})
        })
        .collect();
    systems_with_bodies.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    let mut context = tera::Context::new();
    context.insert("sites", &sites_data);
    context.insert("systems_with_bodies", &systems_with_bodies);

    let html = tmpl.render("admin/sites.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// POST /admin/sites/create (also used by POST /sites/create for Found Colony flow)
pub async fn admin_sites_create(
    simulation: web::Data<SimulationService>,
    form: web::Form<CreateSiteForm>,
) -> Result<HttpResponse> {
    let sys_uuid = Uuid::parse_str(&form.system_id)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid system ID"))?;
    let system_id = StarSystemId(sys_uuid);

    let body_uuid = Uuid::parse_str(&form.body_id)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid body ID"))?;
    let body_id = CelestialBodyId(body_uuid);

    let tick = simulation.current_tick().await;

    simulation.with_state_mut(|state| {
        if let Some(system) = state.galaxy.systems.get_mut(&system_id) {
            let site = match form.site_type.as_str() {
                "installation" => Site::new_installation(body_id, form.name.clone(), tick, form.population),
                _ => Site::new_settlement(body_id, form.name.clone(), tick, form.population),
            };
            system.add_site(site);
        }
    }).await;
    Ok(redirect("/admin/sites"))
}

/// POST /sites/create — Found Colony modal
pub async fn found_colony(
    simulation: web::Data<SimulationService>,
    form: web::Form<CreateSiteForm>,
) -> Result<HttpResponse> {
    let sys_uuid = Uuid::parse_str(&form.system_id)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid system ID"))?;
    let system_id = StarSystemId(sys_uuid);

    let body_uuid = Uuid::parse_str(&form.body_id)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid body ID"))?;
    let body_id = CelestialBodyId(body_uuid);

    let tick = simulation.current_tick().await;

    simulation.with_state_mut(|state| {
        if let Some(system) = state.galaxy.systems.get_mut(&system_id) {
            let site = match form.site_type.as_str() {
                "installation" => Site::new_installation(body_id, form.name.clone(), tick, form.population),
                _ => Site::new_settlement(body_id, form.name.clone(), tick, form.population),
            };
            system.add_site(site);
        }
    }).await;
    Ok(redirect("/colonies"))
}

/// POST /admin/sites/{id}/delete
pub async fn admin_sites_delete(
    simulation: web::Data<SimulationService>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let id_str = path.into_inner();
    if let Ok(uuid) = Uuid::parse_str(&id_str) {
        let site_id = SiteId(uuid);
        simulation.with_state_mut(|state| {
            for system in state.galaxy.systems.values_mut() {
                if system.sites.contains_key(&site_id) {
                    system.remove_site(&site_id);
                    break;
                }
            }
        }).await;
    }
    Ok(redirect("/admin/sites"))
}

/// POST /admin/sites/{id}/edit
pub async fn admin_sites_edit(
    simulation: web::Data<SimulationService>,
    path: web::Path<String>,
    form: web::Form<EditSiteForm>,
) -> Result<HttpResponse> {
    let id_str = path.into_inner();
    if let Ok(uuid) = Uuid::parse_str(&id_str) {
        let site_id = SiteId(uuid);
        let new_name = form.name.clone();
        let new_type = match form.site_type.as_str() {
            "installation" => SiteType::Installation,
            _ => SiteType::Settlement,
        };
        let new_population = form.population;
        simulation.with_state_mut(move |state| {
            for system in state.galaxy.systems.values_mut() {
                if let Some(site) = system.sites.get_mut(&site_id) {
                    site.name = new_name.clone();
                    site.site_type = new_type;
                    site.population.total = new_population;
                    break;
                }
            }
        }).await;
    }
    Ok(redirect("/admin/sites"))
}

// ─── Admin: events ────────────────────────────────────────────────────────────

/// GET /admin/events
pub async fn admin_events_list(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let game_state = simulation.get_state().await;

    let events_data: Vec<_> = game_state.event_log.all().iter()
        .map(|e| json!({
            "index": e.index,
            "tick": e.tick,
            "site_id": e.site_id.to_string(),
            "event_id": &e.event_id,
            "title": &e.title,
            "category": format!("{:?}", e.category),
            "severity": format!("{:?}", e.severity),
            "required_choice": e.required_choice,
        }))
        .collect();

    // Collect sites for the create form
    let sites_data: Vec<_> = game_state.galaxy.systems.values()
        .flat_map(|s| s.sites.values().map(|site| json!({
            "id": site.id.to_string(),
            "name": &site.name,
        })))
        .collect();

    let mut context = tera::Context::new();
    context.insert("events", &events_data);
    context.insert("sites", &sites_data);

    let html = tmpl.render("admin/events.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// POST /admin/events/create
pub async fn admin_events_create(
    simulation: web::Data<SimulationService>,
    form: web::Form<CreateEventForm>,
) -> Result<HttpResponse> {
    let tick = simulation.current_tick().await;
    let severity = parse_severity(&form.severity);

    let site_uuid = Uuid::parse_str(&form.site_id).unwrap_or_default();
    let site_id = SiteId(site_uuid);

    simulation.with_state_mut(|state| {
        state.event_log.push(
            tick,
            site_id,
            form.event_id.clone(),
            form.title.clone(),
            EventCategory::General,
            severity,
            false,
        );
    }).await;
    Ok(redirect("/admin/events"))
}

/// POST /admin/events/{index}/delete
pub async fn admin_events_delete(
    simulation: web::Data<SimulationService>,
    path: web::Path<u64>,
) -> Result<HttpResponse> {
    let index = path.into_inner();
    simulation.with_state_mut(|state| {
        state.event_log.remove_by_index(index);
    }).await;
    Ok(redirect("/admin/events"))
}

// ─── Admin: constructed buildings ────────────────────────────────────────────

/// GET /admin/buildings
pub async fn admin_buildings_list(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let game_state = simulation.get_state().await;

    let mut buildings_data: Vec<serde_json::Value> = Vec::new();
    for sys in game_state.galaxy.systems.values() {
        for site in sys.sites.values() {
            for b in site.buildings.values() {
                let building_name = game_state.building_definitions
                    .get(&b.building_def_id)
                    .map(|def| def.name.clone())
                    .unwrap_or_else(|| b.building_def_id.clone());
                buildings_data.push(json!({
                    "id": b.id.to_string(),
                    "building_def_id": &b.building_def_id,
                    "building_name": building_name,
                    "state": format!("{:?}", b.state),
                    "site_id": site.id.to_string(),
                    "site_name": &site.name,
                    "system_name": &sys.name,
                }));
            }
        }
    }

    let mut context = tera::Context::new();
    context.insert("buildings", &buildings_data);

    let html = tmpl.render("admin/buildings.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// ─── Admin: content resources ────────────────────────────────────────────────

/// GET /admin/content/resources
pub async fn admin_content_resources(
    tmpl: web::Data<tera::Tera>,
    repo: web::Data<SharedRepository>,
) -> Result<HttpResponse> {
    let defs = repo.get_all_resource_defs()
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;

    let mut resources_data: Vec<_> = defs.iter()
        .map(|r| json!({
            "id": &r.id,
            "name": &r.name,
            "category": format!("{:?}", r.category),
            "base_value": r.base_value,
            "tradeable": r.tradeable,
            "consumable": r.consumable,
            "description": &r.description,
            "phase": format!("{:?}", r.storage.phase),
            "density_kg_per_m3": r.density_kg_per_m3,
        }))
        .collect();
    resources_data.sort_by(|a, b| {
        a["id"].as_str().unwrap_or("").cmp(b["id"].as_str().unwrap_or(""))
    });

    let mut context = tera::Context::new();
    context.insert("resources", &resources_data);

    let html = tmpl.render("admin/content_resources.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// GET /admin/content/resources/{id}
pub async fn admin_content_resource_detail(
    repo: web::Data<SharedRepository>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    let def = repo.get_resource_def(&id)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;
    match def {
        Some(d) => Ok(HttpResponse::Ok().content_type("application/json")
            .body(serde_json::to_string(&d).unwrap_or_default())),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

/// POST /admin/content/resources/create
pub async fn admin_content_resources_create(
    repo: web::Data<SharedRepository>,
    simulation: web::Data<SimulationService>,
    form: web::Form<ResourceDefinitionForm>,
) -> Result<HttpResponse> {
    let phase = parse_resource_phase(&form.phase);
    let def = ResourceDefinition {
        id: form.id.clone(),
        name: form.name.clone(),
        description: form.description.clone(),
        category: parse_resource_category(&form.category),
        storage: StorageRequirements {
            phase,
            min_temperature_k: None,
            max_temperature_k: None,
            min_pressure_atm: None,
            max_pressure_atm: None,
            hazardous: false,
            special_handling: None,
        },
        density_kg_per_m3: form.density_kg_per_m3.unwrap_or(1000.0),
        base_value: form.base_value,
        tradeable: form.tradeable.is_some(),
        extractable: false,
        consumable: form.consumable.is_some(),
        stack_size: 0,
    };

    repo.upsert_resource_def(&def, "db")
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;

    let def_clone = def.clone();
    simulation.with_state_mut(move |state| {
        state.resource_definitions.insert(def_clone.id.clone(), def_clone.clone());
    }).await;

    Ok(redirect("/admin/content/resources"))
}

/// POST /admin/content/resources/{id}/edit
pub async fn admin_content_resources_edit(
    repo: web::Data<SharedRepository>,
    simulation: web::Data<SimulationService>,
    path: web::Path<String>,
    form: web::Form<ResourceDefinitionForm>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    let phase = parse_resource_phase(&form.phase);
    let def = ResourceDefinition {
        id: id.clone(),
        name: form.name.clone(),
        description: form.description.clone(),
        category: parse_resource_category(&form.category),
        storage: StorageRequirements {
            phase,
            min_temperature_k: None,
            max_temperature_k: None,
            min_pressure_atm: None,
            max_pressure_atm: None,
            hazardous: false,
            special_handling: None,
        },
        density_kg_per_m3: form.density_kg_per_m3.unwrap_or(1000.0),
        base_value: form.base_value,
        tradeable: form.tradeable.is_some(),
        extractable: false,
        consumable: form.consumable.is_some(),
        stack_size: 0,
    };

    // For db-sourced records we bypass the "don't overwrite db" guard by forcing source="db"
    // and using a raw upsert that always updates.
    {
        let conn_result = repo.upsert_resource_def(&def, "db");
        if let Err(ref _e) = conn_result {
            // Try again — edit should always win
        }
        conn_result.map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;
    }

    let def_clone = def.clone();
    simulation.with_state_mut(move |state| {
        state.resource_definitions.insert(def_clone.id.clone(), def_clone.clone());
    }).await;

    Ok(redirect("/admin/content/resources"))
}

/// POST /admin/content/resources/{id}/delete
pub async fn admin_content_resources_delete(
    repo: web::Data<SharedRepository>,
    simulation: web::Data<SimulationService>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    let _ = repo.delete_resource_def(&id)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;
    let id_clone = id.clone();
    simulation.with_state_mut(move |state| {
        state.resource_definitions.remove(&id_clone);
    }).await;
    Ok(redirect("/admin/content/resources"))
}

// ─── Admin: content buildings ────────────────────────────────────────────────

/// GET /admin/content/buildings
pub async fn admin_content_buildings(
    tmpl: web::Data<tera::Tera>,
    repo: web::Data<SharedRepository>,
) -> Result<HttpResponse> {
    let defs = repo.get_all_building_defs()
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;

    let mut buildings_data: Vec<_> = defs.iter()
        .map(|b| json!({
            "id": &b.id,
            "name": &b.name,
            "category": format!("{:?}", b.category),
            "power_base_mw": b.power.as_ref().map(|p| p.base_mw).unwrap_or(0.0),
            "power_output_mw": b.power_output_mw.unwrap_or(0.0),
            "worker_capacity": b.worker_capacity,
            "description": &b.description,
            "upgradeable": b.upgradeable,
            "morale_bonus": b.morale_bonus,
        }))
        .collect();
    buildings_data.sort_by(|a, b| {
        a["id"].as_str().unwrap_or("").cmp(b["id"].as_str().unwrap_or(""))
    });

    let mut context = tera::Context::new();
    context.insert("buildings", &buildings_data);

    let html = tmpl.render("admin/content_buildings.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// GET /admin/content/buildings/{id}
pub async fn admin_content_building_detail(
    repo: web::Data<SharedRepository>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    let def = repo.get_building_def(&id)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;
    match def {
        Some(d) => Ok(HttpResponse::Ok().content_type("application/json")
            .body(serde_json::to_string(&d).unwrap_or_default())),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

/// POST /admin/content/buildings/create
pub async fn admin_content_buildings_create(
    repo: web::Data<SharedRepository>,
    simulation: web::Data<SimulationService>,
    form: web::Form<BuildingDefinitionForm>,
) -> Result<HttpResponse> {
    let def = BuildingDefinition {
        id: form.id.clone(),
        name: form.name.clone(),
        description: form.description.clone(),
        category: parse_building_category(&form.category),
        construction_costs: vec![],
        construction_time_ticks: 100,
        power: form.power_base_mw.map(|mw| PowerRequirement { base_mw: mw, per_level_mw: 0.0 }),
        power_output_mw: form.power_output_mw,
        worker_capacity: form.worker_capacity,
        storage_capacity: None,
        housing_capacity: None,
        recipes: vec![],
        maintenance_cost: HashMap::new(),
        upgradeable: form.upgradeable.is_some(),
        upgrade_to: None,
        morale_bonus: form.morale_bonus.unwrap_or(0.0),
    };

    repo.upsert_building_def(&def, "db")
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;

    let def_clone = def.clone();
    simulation.with_state_mut(move |state| {
        state.building_definitions.insert(def_clone.id.clone(), def_clone.clone());
    }).await;

    Ok(redirect("/admin/content/buildings"))
}

/// POST /admin/content/buildings/{id}/edit
pub async fn admin_content_buildings_edit(
    repo: web::Data<SharedRepository>,
    simulation: web::Data<SimulationService>,
    path: web::Path<String>,
    form: web::Form<BuildingDefinitionForm>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    // Load existing def to preserve fields not in the form
    let existing = repo.get_building_def(&id)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;

    let base = existing.unwrap_or_else(|| BuildingDefinition {
        id: id.clone(),
        name: String::new(),
        description: String::new(),
        category: BuildingCategory::Infrastructure,
        construction_costs: vec![],
        construction_time_ticks: 100,
        power: None,
        power_output_mw: None,
        worker_capacity: 0,
        storage_capacity: None,
        housing_capacity: None,
        recipes: vec![],
        maintenance_cost: HashMap::new(),
        upgradeable: false,
        upgrade_to: None,
        morale_bonus: 0.0,
    });

    let def = BuildingDefinition {
        id: id.clone(),
        name: form.name.clone(),
        description: form.description.clone(),
        category: parse_building_category(&form.category),
        power: form.power_base_mw.map(|mw| PowerRequirement { base_mw: mw, per_level_mw: 0.0 }),
        power_output_mw: form.power_output_mw,
        worker_capacity: form.worker_capacity,
        upgradeable: form.upgradeable.is_some(),
        morale_bonus: form.morale_bonus.unwrap_or(0.0),
        ..base
    };

    repo.upsert_building_def(&def, "db")
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;

    let def_clone = def.clone();
    simulation.with_state_mut(move |state| {
        state.building_definitions.insert(def_clone.id.clone(), def_clone.clone());
    }).await;

    Ok(redirect("/admin/content/buildings"))
}

/// POST /admin/content/buildings/{id}/delete
pub async fn admin_content_buildings_delete(
    repo: web::Data<SharedRepository>,
    simulation: web::Data<SimulationService>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    let _ = repo.delete_building_def(&id)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;
    let id_clone = id.clone();
    simulation.with_state_mut(move |state| {
        state.building_definitions.remove(&id_clone);
    }).await;
    Ok(redirect("/admin/content/buildings"))
}

// ─── Admin: content recipes ───────────────────────────────────────────────────

/// GET /admin/content/recipes
pub async fn admin_content_recipes(
    tmpl: web::Data<tera::Tera>,
    repo: web::Data<SharedRepository>,
) -> Result<HttpResponse> {
    let all_recipes = repo.get_all_recipes()
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;

    let mut recipes_data: Vec<_> = all_recipes.iter()
        .map(|r| {
            let inputs: Vec<_> = r.inputs.iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect();
            let outputs: Vec<_> = r.outputs.iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect();
            json!({
                "id": &r.id,
                "name": &r.name,
                "building_types": &r.building_types,
                "building_types_str": r.building_types.join(", "),
                "inputs": inputs.join(", "),
                "inputs_json": serde_json::to_string(&r.inputs).unwrap_or_default(),
                "outputs": outputs.join(", "),
                "outputs_json": serde_json::to_string(&r.outputs).unwrap_or_default(),
                "duration_ticks": r.processing_time_ticks,
                "required_deposit": &r.requires_deposit,
            })
        })
        .collect();
    recipes_data.sort_by(|a, b| {
        a["id"].as_str().unwrap_or("").cmp(b["id"].as_str().unwrap_or(""))
    });

    let mut context = tera::Context::new();
    context.insert("recipes", &recipes_data);

    let html = tmpl.render("admin/content_recipes.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// GET /admin/content/recipes/{id}
pub async fn admin_content_recipe_detail(
    repo: web::Data<SharedRepository>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    let recipe = repo.get_recipe(&id)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;
    match recipe {
        Some(r) => Ok(HttpResponse::Ok().content_type("application/json")
            .body(serde_json::to_string(&r).unwrap_or_default())),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

/// POST /admin/content/recipes/create
pub async fn admin_content_recipes_create(
    repo: web::Data<SharedRepository>,
    form: web::Form<RecipeForm>,
) -> Result<HttpResponse> {
    let building_types: Vec<String> = form.building_types
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let inputs: HashMap<String, f64> = serde_json::from_str(&form.inputs_json)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid inputs JSON: {}", e)))?;
    let outputs: HashMap<String, f64> = serde_json::from_str(&form.outputs_json)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid outputs JSON: {}", e)))?;

    let recipe = Recipe {
        id: form.id.clone(),
        name: form.name.clone(),
        building_types,
        inputs,
        outputs,
        processing_time_ticks: form.processing_time_ticks,
        requires_deposit: form.required_deposit.clone().filter(|s| !s.is_empty()),
        description: None,
    };

    repo.upsert_recipe(&recipe, "db")
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;

    Ok(redirect("/admin/content/recipes"))
}

/// POST /admin/content/recipes/{id}/edit
pub async fn admin_content_recipes_edit(
    repo: web::Data<SharedRepository>,
    path: web::Path<String>,
    form: web::Form<RecipeForm>,
) -> Result<HttpResponse> {
    let id = path.into_inner();

    let building_types: Vec<String> = form.building_types
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let inputs: HashMap<String, f64> = serde_json::from_str(&form.inputs_json)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid inputs JSON: {}", e)))?;
    let outputs: HashMap<String, f64> = serde_json::from_str(&form.outputs_json)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid outputs JSON: {}", e)))?;

    let recipe = Recipe {
        id: id.clone(),
        name: form.name.clone(),
        building_types,
        inputs,
        outputs,
        processing_time_ticks: form.processing_time_ticks,
        requires_deposit: form.required_deposit.clone().filter(|s| !s.is_empty()),
        description: None,
    };

    repo.upsert_recipe(&recipe, "db")
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;

    Ok(redirect("/admin/content/recipes"))
}

/// POST /admin/content/recipes/{id}/delete
pub async fn admin_content_recipes_delete(
    repo: web::Data<SharedRepository>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    let _ = repo.delete_recipe(&id)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;
    Ok(redirect("/admin/content/recipes"))
}

// ─── Admin: system detail with bodies/sites CRUD ─────────────────────────────

/// GET /admin/systems/{id}
pub async fn admin_system_detail(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let id_str = path.into_inner();
    let uuid = Uuid::parse_str(&id_str)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid system ID"))?;
    let system_id = StarSystemId(uuid);
    let game_state = simulation.get_state().await;

    let system = game_state.galaxy.systems.get(&system_id)
        .ok_or_else(|| actix_web::error::ErrorNotFound("System not found"))?;

    let system_data = json!({
        "id": system.id.to_string(),
        "name": &system.name,
        "seed": system.seed,
        "explored": system.explored,
    });

    let mut bodies_data: Vec<_> = system.bodies.values()
        .map(|b| json!({
            "id": b.id.to_string(),
            "name": &b.name,
            "body_type": format!("{:?}", b.body_type),
            "gravity": b.gravity,
            "radius_km": b.radius_km,
        }))
        .collect();
    bodies_data.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    let mut sites_data: Vec<_> = system.sites.values()
        .map(|s| {
            let body_name = system.bodies.get(&s.body_id)
                .map(|b| b.name.as_str()).unwrap_or("Unknown");
            json!({
                "id": s.id.to_string(),
                "name": &s.name,
                "site_type": format!("{:?}", s.site_type),
                "body_id": s.body_id.to_string(),
                "body_name": body_name,
                "population": s.population.total,
                "building_count": s.buildings.len(),
            })
        })
        .collect();
    sites_data.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    let body_options: Vec<_> = system.bodies.values()
        .map(|b| json!({"id": b.id.to_string(), "name": &b.name}))
        .collect();

    let mut context = tera::Context::new();
    context.insert("system", &system_data);
    context.insert("bodies", &bodies_data);
    context.insert("sites", &sites_data);
    context.insert("body_options", &body_options);

    let html = tmpl.render("admin/system_detail.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// POST /admin/systems/{id}/bodies/create
pub async fn admin_system_bodies_create(
    simulation: web::Data<SimulationService>,
    path: web::Path<String>,
    form: web::Form<SystemBodyCreateForm>,
) -> Result<HttpResponse> {
    let id_str = path.into_inner();
    let uuid = Uuid::parse_str(&id_str)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid system ID"))?;
    let system_id = StarSystemId(uuid);
    let body_type = parse_body_type(&form.body_type);

    simulation.with_state_mut(move |state| {
        if let Some(system) = state.galaxy.systems.get_mut(&system_id) {
            let body = CelestialBody::new(
                system.id,
                form.name.clone(),
                body_type,
                Atmosphere::None,
                Temperature::Cold,
                0.5,
                3000.0,
                0.3,
            );
            system.add_body(body);
        }
    }).await;
    Ok(redirect(&format!("/admin/systems/{}", id_str)))
}

/// POST /admin/systems/{id}/bodies/{body_id}/edit
pub async fn admin_system_bodies_edit(
    simulation: web::Data<SimulationService>,
    path: web::Path<(String, String)>,
    form: web::Form<SystemBodyEditForm>,
) -> Result<HttpResponse> {
    let (sys_str, body_str) = path.into_inner();
    let sys_uuid = Uuid::parse_str(&sys_str)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid system ID"))?;
    let body_uuid = Uuid::parse_str(&body_str)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid body ID"))?;
    let system_id = StarSystemId(sys_uuid);
    let body_id = CelestialBodyId(body_uuid);
    let body_type = parse_body_type(&form.body_type);
    let new_name = form.name.clone();
    let new_gravity = form.gravity.unwrap_or(0.5);
    let new_radius = form.radius_km.unwrap_or(3000.0);

    simulation.with_state_mut(move |state| {
        if let Some(system) = state.galaxy.systems.get_mut(&system_id) {
            if let Some(body) = system.bodies.get_mut(&body_id) {
                body.name = new_name;
                body.body_type = body_type;
                body.gravity = new_gravity;
                body.radius_km = new_radius;
            }
        }
    }).await;
    Ok(redirect(&format!("/admin/systems/{}", sys_str)))
}

/// POST /admin/systems/{id}/bodies/{body_id}/delete
pub async fn admin_system_bodies_delete(
    simulation: web::Data<SimulationService>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (sys_str, body_str) = path.into_inner();
    let sys_uuid = Uuid::parse_str(&sys_str)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid system ID"))?;
    let body_uuid = Uuid::parse_str(&body_str)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid body ID"))?;
    let system_id = StarSystemId(sys_uuid);
    let body_id = CelestialBodyId(body_uuid);

    simulation.with_state_mut(move |state| {
        if let Some(system) = state.galaxy.systems.get_mut(&system_id) {
            system.remove_body(&body_id);
        }
    }).await;
    Ok(redirect(&format!("/admin/systems/{}", sys_str)))
}

/// POST /admin/systems/{id}/sites/create
pub async fn admin_system_sites_create(
    simulation: web::Data<SimulationService>,
    path: web::Path<String>,
    form: web::Form<SystemSiteCreateForm>,
) -> Result<HttpResponse> {
    let id_str = path.into_inner();
    let sys_uuid = Uuid::parse_str(&id_str)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid system ID"))?;
    let system_id = StarSystemId(sys_uuid);
    let body_uuid = Uuid::parse_str(&form.body_id)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid body ID"))?;
    let body_id = CelestialBodyId(body_uuid);
    let tick = simulation.current_tick().await;

    simulation.with_state_mut(move |state| {
        if let Some(system) = state.galaxy.systems.get_mut(&system_id) {
            let site = match form.site_type.as_str() {
                "installation" => Site::new_installation(body_id, form.name.clone(), tick, form.population),
                _ => Site::new_settlement(body_id, form.name.clone(), tick, form.population),
            };
            system.add_site(site);
        }
    }).await;
    Ok(redirect(&format!("/admin/systems/{}", id_str)))
}

/// POST /admin/systems/{id}/sites/{site_id}/edit
pub async fn admin_system_sites_edit(
    simulation: web::Data<SimulationService>,
    path: web::Path<(String, String)>,
    form: web::Form<SystemSiteEditForm>,
) -> Result<HttpResponse> {
    let (sys_str, site_str) = path.into_inner();
    let sys_uuid = Uuid::parse_str(&sys_str)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid system ID"))?;
    let site_uuid = Uuid::parse_str(&site_str)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid site ID"))?;
    let system_id = StarSystemId(sys_uuid);
    let site_id = SiteId(site_uuid);
    let new_name = form.name.clone();
    let new_population = form.population;

    simulation.with_state_mut(move |state| {
        if let Some(system) = state.galaxy.systems.get_mut(&system_id) {
            if let Some(site) = system.sites.get_mut(&site_id) {
                site.name = new_name;
                site.population.total = new_population as u64;
            }
        }
    }).await;
    Ok(redirect(&format!("/admin/systems/{}", sys_str)))
}

/// POST /admin/systems/{id}/sites/{site_id}/delete
pub async fn admin_system_sites_delete(
    simulation: web::Data<SimulationService>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (sys_str, site_str) = path.into_inner();
    let sys_uuid = Uuid::parse_str(&sys_str)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid system ID"))?;
    let site_uuid = Uuid::parse_str(&site_str)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid site ID"))?;
    let system_id = StarSystemId(sys_uuid);
    let site_id = SiteId(site_uuid);

    simulation.with_state_mut(move |state| {
        if let Some(system) = state.galaxy.systems.get_mut(&system_id) {
            system.remove_site(&site_id);
        }
    }).await;
    Ok(redirect(&format!("/admin/systems/{}", sys_str)))
}
