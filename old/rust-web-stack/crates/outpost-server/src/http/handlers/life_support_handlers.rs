//! Life support HTTP handlers — MVP.5.
//!
//! Provides:
//! - GET /site/{id}/life-support → life support tab partial

use actix_web::{web, HttpResponse, Result};
use outpost_core::domain::SiteId;
use outpost_core::domain::life_support::LifeSupportDemand;
use outpost_core::domain::ResourceType;
use uuid::Uuid;

use crate::services::SimulationService;

/// GET /site/{id}/life-support
///
/// Render the Life Support tab partial for a site.
pub async fn site_life_support_tab(
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

    let pop = site.population.total;
    let ls = &site.life_support;

    // Per-tick demand (for display)
    let oxygen_demand = LifeSupportDemand::oxygen(pop);
    let water_demand  = LifeSupportDemand::water(pop);
    let food_demand   = LifeSupportDemand::food(pop);

    // Current stockpile
    let oxygen_stock = site.resources.get(ResourceType::Oxygen);
    let water_stock  = site.resources.get(ResourceType::Water);
    let food_stock   = site.resources.get(ResourceType::Food);

    // Ticks until depletion at current demand
    let ticks_until_oxygen_empty = if oxygen_demand > 0.0 && ls.oxygen_sat < 1.0 {
        (oxygen_stock as f64 / oxygen_demand).floor() as i64
    } else if oxygen_demand > 0.0 {
        (oxygen_stock as f64 / oxygen_demand).floor() as i64
    } else { -1 };

    let ticks_until_water_empty = if water_demand > 0.0 {
        (water_stock as f64 / water_demand).floor() as i64
    } else { -1 };

    let ticks_until_food_empty = if food_demand > 0.0 {
        (food_stock as f64 / food_demand).floor() as i64
    } else { -1 };

    let oxygen_sat_pct = (ls.oxygen_sat * 100.0).round() as u32;
    let water_sat_pct  = (ls.water_sat  * 100.0).round() as u32;
    let food_sat_pct   = (ls.food_sat   * 100.0).round() as u32;
    let overall_score_pct = (ls.overall_score() * 100.0).round() as u32;

    let mut context = tera::Context::new();
    context.insert("site_id", &site_id_str);
    context.insert("population", &pop);
    context.insert("oxygen_sat",    &oxygen_sat_pct);
    context.insert("water_sat",     &water_sat_pct);
    context.insert("food_sat",      &food_sat_pct);
    context.insert("is_critical",   &ls.is_critical);
    context.insert("overall_score", &overall_score_pct);
    context.insert("status_label", ls.status_label());
    context.insert("oxygen_demand", &oxygen_demand);
    context.insert("water_demand",  &water_demand);
    context.insert("food_demand",   &food_demand);
    context.insert("oxygen_stock",  &oxygen_stock);
    context.insert("water_stock",   &water_stock);
    context.insert("food_stock",    &food_stock);
    context.insert("ticks_oxygen",  &ticks_until_oxygen_empty);
    context.insert("ticks_water",   &ticks_until_water_empty);
    context.insert("ticks_food",    &ticks_until_food_empty);

    let html = tmpl.render("components/_life_support_tab.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
