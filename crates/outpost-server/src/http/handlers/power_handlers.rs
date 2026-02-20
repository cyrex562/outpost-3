//! Power grid HTTP handlers — MVP.5.
//!
//! Provides:
//! - GET  /site/{id}/power                  → full power tab partial
//! - POST /site/{id}/building/{bid}/power/toggle → toggle building power

use actix_web::{web, HttpResponse, Result};
use outpost_core::domain::{BuildingId, SiteId};
use outpost_core::domain::building_instance::BuildingState;
use outpost_core::commands::{ToggleBuildingPower, PowerToggleError};
use outpost_core::simulation::{compute_generation, compute_consumption};
use uuid::Uuid;

use crate::services::SimulationService;

// ─────────────────────────────────────────────────────────────────────────────
// GET /site/{id}/power
// ─────────────────────────────────────────────────────────────────────────────

/// Render the Power tab partial for a site.
pub async fn site_power_tab(
    path: web::Path<String>,
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let site_id_str = path.into_inner();
    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);

    let game_state = simulation.get_state().await;

    let site = game_state.galaxy.find_site(&site_id)
        .ok_or_else(|| actix_web::error::ErrorNotFound("Site not found"))?;

    // Per-building power breakdown
    let mut generators: Vec<serde_json::Value> = Vec::new();
    let mut consumers: Vec<serde_json::Value> = Vec::new();

    for building in site.buildings.values() {
        let Some(def) = game_state.building_definitions.get(&building.building_def_id) else { continue };

        let is_operational = building.state.can_operate();
        let state_label = match building.state {
            BuildingState::Operational => "Operational",
            BuildingState::Paused => "Paused",
            BuildingState::Damaged { severity } if severity < 50 => "Minor Damage",
            BuildingState::Damaged { .. } => "Damaged",
            BuildingState::UnderConstruction { .. } => "Under Construction",
            BuildingState::Destroyed => "Destroyed",
        };

        if let Some(mw) = def.power_output_mw {
            generators.push(serde_json::json!({
                "building_id": building.id.to_string(),
                "name": def.name,
                "mw": mw,
                "operational": is_operational,
                "state": state_label,
                "is_paused": matches!(building.state, BuildingState::Paused),
            }));
        }

        if let Some(power) = &def.power {
            consumers.push(serde_json::json!({
                "building_id": building.id.to_string(),
                "name": def.name,
                "mw": power.base_mw,
                "operational": is_operational,
                "state": state_label,
                "is_paused": matches!(building.state, BuildingState::Paused),
                "category": format!("{:?}", def.category),
            }));
        }
    }

    // Sort: operational first, then alphabetically
    generators.sort_by(|a, b| {
        let a_op = a["operational"].as_bool().unwrap_or(false);
        let b_op = b["operational"].as_bool().unwrap_or(false);
        b_op.cmp(&a_op).then(a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")))
    });
    consumers.sort_by(|a, b| {
        let a_op = a["operational"].as_bool().unwrap_or(false);
        let b_op = b["operational"].as_bool().unwrap_or(false);
        b_op.cmp(&a_op).then(a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")))
    });

    let gen_total: f64 = generators.iter()
        .filter(|g| g["operational"].as_bool().unwrap_or(false))
        .filter_map(|g| g["mw"].as_f64())
        .sum();

    let con_total: f64 = consumers.iter()
        .filter(|c| c["operational"].as_bool().unwrap_or(false))
        .filter_map(|c| c["mw"].as_f64())
        .sum();

    let net_mw = gen_total - con_total;
    let has_brownout = net_mw < 0.0;
    let utilization_pct = if gen_total > 0.0 { (con_total / gen_total * 100.0).clamp(0.0, 100.0) } else { 100.0 };
    let power_efficiency = (site.power_efficiency * 100.0).round() as u32;

    let mut context = tera::Context::new();
    context.insert("site_id", &site_id_str);
    context.insert("generators", &generators);
    context.insert("consumers", &consumers);
    context.insert("gen_total", &gen_total);
    context.insert("con_total", &con_total);
    context.insert("net_mw", &net_mw);
    context.insert("has_brownout", &has_brownout);
    context.insert("utilization_pct", &(utilization_pct.round() as u32));
    context.insert("power_efficiency", &power_efficiency);

    let html = tmpl.render("components/_power_tab.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /site/{id}/building/{bid}/power/toggle
// ─────────────────────────────────────────────────────────────────────────────

/// Toggle a building's power state (Operational ↔ Paused).
pub async fn toggle_building_power(
    path: web::Path<(String, String)>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let (site_id_str, bid_str) = path.into_inner();

    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);
    let building_id = BuildingId(Uuid::parse_str(&bid_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid building ID: {}", e)))?);

    let mut game_state = simulation.get_state().await;

    // Locate the building and determine the desired transition
    let current_state = {
        let site = game_state.galaxy.find_site(&site_id)
            .ok_or_else(|| actix_web::error::ErrorNotFound("Site not found"))?;
        let building = site.buildings.get(&building_id)
            .ok_or_else(|| actix_web::error::ErrorNotFound("Building not found"))?;
        building.state
    };

    // Decide: if currently operational/damaged → power off; if paused → power on
    let power_on = matches!(current_state, BuildingState::Paused);
    let cmd = if power_on {
        ToggleBuildingPower::power_on(site_id, building_id)
    } else {
        ToggleBuildingPower::power_off(site_id, building_id)
    };

    let new_state = cmd.apply(current_state)
        .map_err(|e| match e {
            PowerToggleError::AlreadyInState { .. } =>
                actix_web::error::ErrorConflict(e.to_string()),
            PowerToggleError::UnderConstruction(_) | PowerToggleError::Destroyed(_) =>
                actix_web::error::ErrorUnprocessableEntity(e.to_string()),
            _ => actix_web::error::ErrorBadRequest(e.to_string()),
        })?;

    // Apply the state change to actual game state
    simulation.apply_power_toggle(site_id, building_id, new_state).await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Return HX-Trigger header so the power tab refreshes
    Ok(HttpResponse::Ok()
        .insert_header(("HX-Trigger", "power-changed"))
        .body(format!(r#"<span class="toggle-result">{}</span>"#,
            if power_on { "Powered On" } else { "Powered Off" })))
}
