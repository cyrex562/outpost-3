//! Resource API handlers — lightweight polling endpoints for the top bar.

use actix_web::{web, HttpResponse, Result};
use outpost_core::domain::ResourceType;
use outpost_core::simulation::{compute_site_storage, compute_resource_rates};

use crate::services::SimulationService;

/// GET /api/resources/summary
///
/// Returns a small HTML partial showing aggregate per-resource counts across
/// every site in the galaxy. Used by HTMX polling in the top navigation bar.
pub async fn resource_summary(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let game_state = simulation.get_state().await;
    let content = simulation.get_content().await;

    // Aggregate critical resources across all sites
    let mut total_food: i64 = 0;
    let mut total_water: i64 = 0;
    let mut total_oxygen: i64 = 0;
    let mut total_storage_used: f64 = 0.0;
    let mut total_storage_cap: f64 = 0.0;
    let mut site_count = 0usize;

    // Aggregate net rates (for trend arrows)
    let mut net_food:   f64 = 0.0;
    let mut net_water:  f64 = 0.0;
    let mut net_oxygen: f64 = 0.0;

    for site in game_state.galaxy.all_sites() {
        total_food   += site.resources.get(ResourceType::Food);
        total_water  += site.resources.get(ResourceType::Water);
        total_oxygen += site.resources.get(ResourceType::Oxygen);

        let cap = compute_site_storage(site, &game_state.building_definitions);
        let used: i64 = site.resources.iter().map(|(_, q)| q).sum();
        total_storage_used += used as f64;
        total_storage_cap  += cap;
        site_count += 1;

        let rates = compute_resource_rates(site, &game_state.building_definitions, &content);
        net_food   += rates.get("food").map(|r| r.net()).unwrap_or(0.0);
        net_water  += rates.get("water").map(|r| r.net()).unwrap_or(0.0);
        net_oxygen += rates.get("oxygen").map(|r| r.net()).unwrap_or(0.0);
    }

    let storage_pct = if total_storage_cap > 0.0 {
        (total_storage_used / total_storage_cap * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    let food_status   = resource_status(total_food);
    let water_status  = resource_status(total_water);
    let oxygen_status = resource_status(total_oxygen);

    let mut context = tera::Context::new();
    context.insert("food",          &total_food);
    context.insert("water",         &total_water);
    context.insert("oxygen",        &total_oxygen);
    context.insert("food_status",   &food_status);
    context.insert("water_status",  &water_status);
    context.insert("oxygen_status", &oxygen_status);
    context.insert("food_trend",    &trend_arrow(net_food));
    context.insert("water_trend",   &trend_arrow(net_water));
    context.insert("oxygen_trend",  &trend_arrow(net_oxygen));
    context.insert("storage_pct",   &(storage_pct.round() as u32));
    context.insert("storage_status",&storage_status(storage_pct));
    context.insert("site_count",    &site_count);

    let html = tmpl
        .render("components/_resource_summary.html", &context)
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Template error: {}", e))
        })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

fn resource_status(qty: i64) -> &'static str {
    if qty <= 0      { "critical" }
    else if qty < 50 { "warning"  }
    else             { "good"     }
}

fn storage_status(pct: f64) -> &'static str {
    if pct >= 90.0      { "critical" }
    else if pct >= 70.0 { "warning"  }
    else                { "good"     }
}

/// Return a trend arrow character based on the net rate.
fn trend_arrow(net: f64) -> &'static str {
    if net > 0.01      { "▲" }
    else if net < -0.01 { "▼" }
    else               { "—" }
}
