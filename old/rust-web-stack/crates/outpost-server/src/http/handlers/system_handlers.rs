//! System list and detail handlers.

use actix_web::{web, HttpResponse, Result};
use serde_json::json;
use uuid::Uuid;

use outpost_core::domain::StarSystemId;
use crate::services::SimulationService;

/// GET /systems — list all star systems.
pub async fn systems_list(
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
                "seed": s.seed,
                "body_count": s.bodies.len(),
                "site_count": s.sites.len(),
                "total_population": s.total_population(),
            })
        })
        .collect();

    systems_data.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    let total_bodies: usize = game_state.galaxy.systems.values().map(|s| s.bodies.len()).sum();
    let explored_count = game_state.galaxy.systems.values().filter(|s| s.explored).count();

    let mut context = tera::Context::new();
    context.insert("systems", &systems_data);
    context.insert("total_systems", &systems_data.len());
    context.insert("explored_count", &explored_count);
    context.insert("unexplored_count", &(systems_data.len() - explored_count));
    context.insert("total_bodies", &total_bodies);

    let html = tmpl.render("systems.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// GET /systems/{id} — detail for a single star system.
pub async fn system_detail(
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

    let bodies_data: Vec<_> = system.bodies.values()
        .map(|b| {
            let sites_count = system.site_count_on_body(&b.id);
            json!({
                "id": b.id.to_string(),
                "name": &b.name,
                "body_type": format!("{:?}", b.body_type),
                "atmosphere_type": match &b.atmosphere {
                    outpost_core::domain::Atmosphere::None => "None",
                    outpost_core::domain::Atmosphere::Trace => "Trace",
                    outpost_core::domain::Atmosphere::Thin { .. } => "Thin",
                    outpost_core::domain::Atmosphere::Breathable { .. } => "Breathable",
                    outpost_core::domain::Atmosphere::Dense { .. } => "Dense",
                    outpost_core::domain::Atmosphere::Crushing { .. } => "Crushing",
                    outpost_core::domain::Atmosphere::Toxic { .. } => "Toxic",
                },
                "temperature": format!("{:?}", b.temperature),
                "gravity": b.gravity,
                "radius_km": b.radius_km,
                "sites_count": sites_count,
            })
        })
        .collect();

    let sites_data: Vec<_> = system.sites.values()
        .map(|s| {
            let body_name = system.bodies.get(&s.body_id)
                .map(|b| b.name.as_str())
                .unwrap_or("Unknown");
            json!({
                "id": s.id.to_string(),
                "name": &s.name,
                "site_type": format!("{:?}", s.site_type),
                "body_name": body_name,
                "population": s.population.total,
                "building_count": s.buildings.len(),
            })
        })
        .collect();

    let mut context = tera::Context::new();
    context.insert("system_id", &id_str);
    context.insert("system_name", &system.name);
    context.insert("explored", &system.explored);
    context.insert("seed", &system.seed);
    context.insert("bodies", &bodies_data);
    context.insert("sites", &sites_data);

    let html = tmpl.render("system_detail.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
