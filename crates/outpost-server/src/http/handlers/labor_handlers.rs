//! Labor management handlers for the V5 Site Detail — Labor tab.
//!
//! Provides:
//! - GET /site/{id}/labor — Labor tab partial
//! - POST /site/{id}/building/{bid}/assign — assign workers to building
//! - POST /site/{id}/building/{bid}/deallocate — remove workers from building

use actix_web::{web, HttpResponse, Result};
use outpost_core::domain::{BuildingId, SiteId, SkillCategory};
use outpost_core::commands::{AssignLabor, DeallocateLabor};
use serde::Deserialize;
use uuid::Uuid;

use crate::services::SimulationService;

/// GET /site/{site_id}/labor
///
/// Render the labor tab partial (HTMX target).
pub async fn site_labor_tab(
    path: web::Path<String>,
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let site_id_str = path.into_inner();
    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);

    let game_state = simulation.get_state().await;

    let site = game_state.galaxy.find_site(&site_id)
        .ok_or_else(|| actix_web::error::ErrorNotFound(format!("Site {} not found", site_id)))?;

    // Build labor pool by skill
    let labor_pool: Vec<serde_json::Value> = SkillCategory::all().iter().map(|skill| {
        let available = site.population.available_by_skill(*skill);
        serde_json::json!({
            "skill": skill.label(),
            "available": available,
        })
    }).collect();

    // Build building assignments list (for buildings with worker capacity)
    let building_assignments: Vec<serde_json::Value> = site.buildings.values()
        .filter_map(|building| {
            let def = game_state.building_definitions.get(&building.building_def_id)?;
            if def.worker_capacity == 0 {
                return None; // Skip buildings with no labor requirement
            }
            let efficiency_pct = if def.worker_capacity > 0 {
                ((building.workers_assigned as f64 / def.worker_capacity as f64) * 100.0)
                    .min(100.0) as u32
            } else {
                100
            };

            Some(serde_json::json!({
                "id": building.id.to_string(),
                "name": def.name,
                "category": format!("{:?}", def.category),
                "state": format!("{:?}", building.state),
                "workers_assigned": building.workers_assigned,
                "worker_capacity": def.worker_capacity,
                "efficiency_pct": efficiency_pct,
                "is_operational": building.state.can_operate(),
            }))
        })
        .collect();

    // Build character roster
    let characters: Vec<serde_json::Value> = site.characters.iter().map(|c| {
        serde_json::json!({
            "id": c.id.to_string(),
            "name": c.name,
            "age": c.age,
            "primary_skill": c.primary_skill.label(),
            "morale": c.morale.round() as u32,
            "health": c.health.round() as u32,
            "is_assigned": c.is_assigned(),
            "assigned_building_id": c.assigned_building_id.map(|id| id.to_string()),
            "traits": c.traits,
        })
    }).collect();

    // Summary stats
    let total_workers = site.population.total;
    let employed = site.population.employed;
    let unemployed = site.population.unemployed;
    let morale = site.population.morale.round() as u32;

    // Needs satisfaction for display
    let needs = &site.population.needs;
    let food_pct = (needs.food * 100.0) as u32;
    let water_pct = (needs.water * 100.0) as u32;
    let oxygen_pct = (needs.oxygen * 100.0) as u32;

    let growth_rate_pct = (site.population.growth_rate * 100.0 * 10.0).round() / 10.0;

    let mut ctx = tera::Context::new();
    ctx.insert("site_id", &site_id_str);
    ctx.insert("labor_pool", &labor_pool);
    ctx.insert("building_assignments", &building_assignments);
    ctx.insert("characters", &characters);
    ctx.insert("total_workers", &total_workers);
    ctx.insert("employed", &employed);
    ctx.insert("unemployed", &unemployed);
    ctx.insert("morale", &morale);
    ctx.insert("food_pct", &food_pct);
    ctx.insert("water_pct", &water_pct);
    ctx.insert("oxygen_pct", &oxygen_pct);
    ctx.insert("growth_rate_pct", &growth_rate_pct);

    let html = tmpl.render("components/_labor_tab.html", &ctx)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Form data for labor assignment.
#[derive(Deserialize)]
pub struct LaborAssignForm {
    /// Skill type of workers to assign (e.g., "Laborer").
    pub skill: String,
    /// Number of workers to assign.
    pub count: u32,
}

/// POST /site/{site_id}/building/{building_id}/assign
///
/// Assign workers to a building and return updated labor tab.
pub async fn assign_labor(
    path: web::Path<(String, String)>,
    form: web::Form<LaborAssignForm>,
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let (site_id_str, building_id_str) = path.into_inner();
    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);
    let building_id = BuildingId(Uuid::parse_str(&building_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid building ID: {}", e)))?);

    let skill = parse_skill(&form.skill)
        .ok_or_else(|| actix_web::error::ErrorBadRequest(format!("Unknown skill: {}", form.skill)))?;

    let mut game_state = simulation.get_state().await;

    let command = AssignLabor {
        site_id,
        building_id,
        skill,
        count: form.count,
    };

    command.execute(&mut game_state)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Cannot assign labor: {}", e)))?;

    // Return updated labor tab
    site_labor_tab(
        web::Path::from(site_id_str),
        tmpl,
        simulation,
    ).await
}

/// Form data for labor deallocation.
#[derive(Deserialize)]
pub struct LaborDeallocateForm {
    pub skill: String,
    pub count: u32,
}

/// POST /site/{site_id}/building/{building_id}/deallocate
///
/// Remove workers from a building and return them to pool.
pub async fn deallocate_labor(
    path: web::Path<(String, String)>,
    form: web::Form<LaborDeallocateForm>,
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let (site_id_str, building_id_str) = path.into_inner();
    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);
    let building_id = BuildingId(Uuid::parse_str(&building_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid building ID: {}", e)))?);

    let skill = parse_skill(&form.skill)
        .ok_or_else(|| actix_web::error::ErrorBadRequest(format!("Unknown skill: {}", form.skill)))?;

    let mut game_state = simulation.get_state().await;

    let command = DeallocateLabor {
        site_id,
        building_id,
        skill,
        count: form.count,
    };

    command.execute(&mut game_state)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Cannot deallocate labor: {}", e)))?;

    // Return updated labor tab
    site_labor_tab(
        web::Path::from(site_id_str),
        tmpl,
        simulation,
    ).await
}

/// Parse skill string from form data into SkillCategory.
fn parse_skill(s: &str) -> Option<SkillCategory> {
    match s {
        "Laborer" => Some(SkillCategory::Laborer),
        "Engineer" => Some(SkillCategory::Engineer),
        "Scientist" => Some(SkillCategory::Scientist),
        "Farmer" => Some(SkillCategory::Farmer),
        "Medic" => Some(SkillCategory::Medic),
        "Operator" => Some(SkillCategory::Operator),
        _ => None,
    }
}
