//! Colonies list handler for V5 architecture.
//!
//! Provides the colonies overview page showing all sites across the galaxy.

use actix_web::{web, HttpResponse, Result};
use outpost_core::domain::{BuildingStateV5};
use serde_json::json;

use crate::services::SimulationService;

/// GET /colonies
/// 
/// Render the colonies overview page with a table of all sites.
pub async fn colonies_list(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    // Get game state
    let game_state = simulation.get_state().await;
    
    // Collect all sites with their metadata
    let mut sites_data = Vec::new();
    let mut total_population = 0u64;
    let mut total_morale = 0f64;
    let mut total_buildings = 0usize;
    let mut morale_count = 0usize;
    
    for system in game_state.galaxy.systems.values() {
        for site in system.sites.values() {
            // Find the parent body for this site
            let body = system.bodies.get(&site.body_id)
                .expect("Site's parent body should exist");
            
            // Calculate power stats for this site
            let power_generation: i64 = site.buildings.values()
                .filter(|b| b.state == BuildingStateV5::Operational)
                .filter_map(|b| game_state.building_definitions.get(&b.building_def_id))
                .filter_map(|def| def.power_output_mw)
                .map(|mw| mw as i64)
                .sum();
            
            let power_consumption: i64 = site.buildings.values()
                .filter(|b| b.state == BuildingStateV5::Operational)
                .filter_map(|b| game_state.building_definitions.get(&b.building_def_id))
                .filter_map(|def| def.power.as_ref())
                .map(|power_req| power_req.base_mw as i64)
                .sum();
            
            let power_net = power_generation - power_consumption;
            
            // Calculate morale (placeholder - will be real calculation in future)
            let morale = 75.0;
            
            // Aggregate stats
            total_population += site.population.total;
            total_morale += morale;
            morale_count += 1;
            total_buildings += site.buildings.len();
            
            sites_data.push(json!({
                "id": site.id.to_string(),
                "name": &site.name,
                "site_type": format!("{:?}", site.site_type),
                "body_name": &body.name,
                "system_name": &system.name,
                "population": site.population.total,
                "morale": morale,
                "power_net": power_net,
                "building_count": site.buildings.len(),
                "construction_count": site.construction_queue.all_jobs().len(),
            }));
        }
    }
    
    // Calculate averages
    let average_morale = if morale_count > 0 {
        total_morale / morale_count as f64
    } else {
        0.0
    };
    
    // Sort sites by name
    sites_data.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });

    // Collect systems for "Found Colony" modal — include bodies per system
    let mut all_systems: Vec<_> = game_state.galaxy.systems.values()
        .map(|s| {
            let mut bodies: Vec<_> = s.bodies.values()
                .map(|b| json!({"id": b.id.to_string(), "name": &b.name, "body_type": format!("{:?}", b.body_type)}))
                .collect();
            bodies.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
            json!({"id": s.id.to_string(), "name": &s.name, "bodies": bodies})
        })
        .collect();
    all_systems.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    // Prepare context
    let mut context = tera::Context::new();
    context.insert("sites", &sites_data);
    context.insert("total_sites", &sites_data.len());
    context.insert("total_population", &total_population);
    context.insert("average_morale", &average_morale);
    context.insert("total_buildings", &total_buildings);
    context.insert("all_systems", &all_systems);
    
    let html = tmpl.render("colonies.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
