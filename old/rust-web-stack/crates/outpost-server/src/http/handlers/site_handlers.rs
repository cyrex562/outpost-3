//! Site detail handlers for V5 architecture.
//!
//! These handlers provide:
//! - Full site detail page with tabbed layout
//! - HTMX partials for buildings and construction tabs
//! - Actions: build new structures, pause/cancel construction

use actix_web::{web, HttpResponse, Result};
use outpost_core::domain::{BuildingId, SiteId, BuildingStateV5};
use outpost_core::domain::resource_mapping::{resource_type_to_string, string_to_resource_type};
use outpost_core::commands::{StartConstruction, CancelConstruction, PauseConstruction, ResumeConstruction};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

use crate::services::SimulationService;

/// GET /site/{site_id}
/// 
/// Render the full site detail page with all data and tabs.
pub async fn site_detail(
    path: web::Path<String>,
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let site_id_str = path.into_inner();
    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);
    
    // Get game state
    let game_state = simulation.get_state().await;
    
    // Find the site
    let site = game_state.galaxy.find_site(&site_id)
        .ok_or_else(|| actix_web::error::ErrorNotFound(format!("Site {} not found", site_id)))?;
    
    // Find parent body and system using the site's body_id
    let (body, system) = game_state.galaxy.systems.values()
        .find_map(|system| {
            system.bodies.get(&site.body_id)
                .map(|body| (body, system))
        })
        .ok_or_else(|| actix_web::error::ErrorNotFound("Parent body not found"))?;
    
    // Calculate power stats
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
    let has_brownout = power_net < 0;
    let power_utilization_pct = if power_generation > 0 {
        ((power_consumption as f64 / power_generation as f64) * 100.0).clamp(0.0, 100.0) as u32
    } else if power_consumption > 0 { 100 } else { 0 };

    // Read cached life support status (updated each tick; safe to read without side effects)
    let ls = &site.life_support;
    let ls_oxygen_pct = (ls.oxygen_sat * 100.0).round() as u32;
    let ls_water_pct  = (ls.water_sat  * 100.0).round() as u32;
    let ls_food_pct   = (ls.food_sat   * 100.0).round() as u32;
    let ls_overall    = (ls.overall_score() * 100.0).round() as u32;
    let ls_status = if ls.is_critical { "Critical" }
                    else if ls.overall_score() < 0.75 { "Warning" }
                    else { "Good" };

    // Calculate storage (placeholder - will be implemented in MVP.3)
    let storage_capacity = 1000;
    let storage_used = 450;
    
    // Calculate labor stats (placeholder - will be improved in MVP.4)
    let available_labor = (site.population.total as f64 * 0.1) as u64;
    let assigned_workers: u32 = site.buildings.values()
        .filter(|b| b.state == BuildingStateV5::Operational)
        .map(|b| b.workers_assigned)
        .sum();
    
    // Prepare context
    let mut context = tera::Context::new();
    
    context.insert("site", &serde_json::json!({
        "id": site_id.to_string(),
        "name": &site.name,
        "site_type": format!("{:?}", site.site_type),
        "population": site.population.total,
        "morale": site.population.morale.round() as u32,
        "power_generation": power_generation,
        "power_consumption": power_consumption,
        "power_net": power_net,
        "power_capacity": power_generation,
        "has_brownout": has_brownout,
        "power_utilization_pct": power_utilization_pct,
        "building_count": site.buildings.len(),
        "construction_count": site.construction_queue.all_jobs().len(),
        "founded_at_tick": site.founded_at_tick,
        "pollution_level": site.pollution_level,
        "storage_capacity": storage_capacity,
        "storage_used": storage_used,
        "available_labor": available_labor,
        "assigned_workers": assigned_workers,
        "ls_oxygen_pct": ls_oxygen_pct,
        "ls_water_pct": ls_water_pct,
        "ls_food_pct": ls_food_pct,
        "ls_overall": ls_overall,
        "ls_status": ls_status,
        "ls_is_critical": ls.is_critical,
    }));
    
    context.insert("body", &serde_json::json!({
        "name": &body.name,
    }));
    
    context.insert("system", &serde_json::json!({
        "name": &system.name,
    }));
    
    let html = tmpl.render("site_detail.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// GET /site/{site_id}/buildings
///
/// Render the buildings tab partial (HTMX target)
pub async fn site_buildings_tab(
    path: web::Path<String>,
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let site_id_str = path.into_inner();
    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);
    
    // Get game state
    let game_state = simulation.get_state().await;
    
    // Find the site
    let site = game_state.galaxy.find_site(&site_id)
        .ok_or_else(|| actix_web::error::ErrorNotFound(format!("Site {} not found", site_id)))?;
    
    // Build building list with definitions
    let buildings: Vec<serde_json::Value> = site.buildings.values()
        .filter_map(|building| {
            let def = game_state.building_definitions.get(&building.building_def_id)?;
            
            let (efficiency, state_label, state_badge_class, is_operational, is_paused, can_toggle, can_demolish) = match building.state {
                BuildingStateV5::Operational => (100i64, "Operational", "badge-success", true, false, true, true),
                BuildingStateV5::Paused => (0i64, "Paused", "badge-warning", false, true, true, true),
                BuildingStateV5::Damaged { severity } => ((100 - severity) as i64, "Damaged", "badge-error", false, false, true, true),
                BuildingStateV5::UnderConstruction { .. } => (0i64, "Building", "badge-info", false, false, false, false),
                BuildingStateV5::Destroyed => (0i64, "Destroyed", "badge-ghost", false, false, false, true),
            };
            
            // Calculate current output (placeholder)
            let current_output = if building.state == BuildingStateV5::Operational {
                if let Some(recipe) = def.recipes.first() {
                    let total_output: i64 = recipe.outputs.iter()
                        .map(|output| output.amount_per_tick as i64)
                        .sum();
                    format!("{} units/tick", total_output)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            
            // Get power stats
            let power_consumption = def.power.as_ref().map(|p| p.base_mw as i64).unwrap_or(0);
            let power_generation = def.power_output_mw.map(|mw| mw as i64).unwrap_or(0);
            
            Some(serde_json::json!({
                "id": building.id.to_string(),
                "name": &def.name,
                "category": format!("{:?}", def.category),
                "state_label": state_label,
                "state_badge_class": state_badge_class,
                "is_operational": is_operational,
                "is_paused": is_paused,
                "can_toggle": can_toggle,
                "can_demolish": can_demolish,
                "workers_assigned": building.workers_assigned,
                "workers_capacity": def.worker_capacity,
                "efficiency": efficiency,
                "power_consumption": power_consumption,
                "power_generation": power_generation,
                "current_output": current_output,
            }))
        })
        .collect();
    
    let operational_count = buildings.iter()
        .filter(|b| b["is_operational"].as_bool() == Some(true))
        .count();

    // Build available resources map for build menu affordability checks
    let available_resources: HashMap<String, i64> = site.resources.iter()
        .map(|(rt, &amount)| (resource_type_to_string(*rt).to_string(), amount))
        .collect();

    let building_defs: Vec<serde_json::Value> = game_state.building_definitions.values()
        .map(|def| {
            let costs: Vec<serde_json::Value> = def.construction_costs.iter()
                .map(|c| serde_json::json!({
                    "resource_id": &c.resource_id,
                    "amount": c.amount,
                    "available": available_resources.get(&c.resource_id).copied().unwrap_or(0),
                    "can_afford": available_resources.get(&c.resource_id).copied().unwrap_or(0) >= c.amount as i64,
                }))
                .collect();
            let can_afford = costs.iter().all(|c| c["can_afford"].as_bool().unwrap_or(false));
            serde_json::json!({
                "id": &def.id,
                "name": &def.name,
                "category": format!("{:?}", def.category),
                "description": &def.description,
                "worker_capacity": def.worker_capacity,
                "power_consumption": def.power.as_ref().map(|p| p.base_mw).unwrap_or(0.0),
                "power_generation": def.power_output_mw.unwrap_or(0.0),
                "storage_capacity": def.storage_capacity.unwrap_or(0.0),
                "construction_time_ticks": def.construction_time_ticks,
                "construction_costs": costs,
                "can_afford": can_afford,
            })
        })
        .collect();
    let mut categories: Vec<String> = building_defs.iter()
        .filter_map(|d| d["category"].as_str().map(|s| s.to_string()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    categories.sort();

    let mut context = tera::Context::new();
    context.insert("site_id", &site_id.to_string());
    context.insert("buildings", &buildings);
    context.insert("operational_count", &operational_count);
    context.insert("building_definitions", &building_defs);
    context.insert("categories", &categories);
    
    let html = tmpl.render("components/_buildings_tab.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// GET /site/{site_id}/construction
///
/// Render the construction queue tab partial (HTMX target)
pub async fn site_construction_tab(
    path: web::Path<String>,
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let site_id_str = path.into_inner();
    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);
    
    // Get game state
    let game_state = simulation.get_state().await;
    
    // Find the site
    let site = game_state.galaxy.find_site(&site_id)
        .ok_or_else(|| actix_web::error::ErrorNotFound(format!("Site {} not found", site_id)))?;
    
    // Build construction jobs list
    let construction_jobs: Vec<serde_json::Value> = site.construction_queue.all_jobs()
        .iter()
        .filter_map(|job| {
            let def = game_state.building_definitions.get(&job.building_def_id)?;
            
            let progress_percentage = job.progress_percentage();
            let estimated_completion_tick = job.started_at_tick + job.total_ticks_required;
            
            Some(serde_json::json!({
                "building_id": job.id.to_string(),
                "building_name": &def.name,
                "category": format!("{:?}", def.category),
                "progress_ticks": job.progress_ticks,
                "total_ticks": job.total_ticks_required,
                "progress_percentage": progress_percentage,
                "workers_assigned": job.workers_assigned,
                "started_at_tick": job.started_at_tick,
                "estimated_completion_tick": estimated_completion_tick,
                "paused": job.paused,
                "waste_percentage": 20, // Default waste percentage for cancellation
            }))
        })
        .collect();
    
    let active_count = construction_jobs.iter()
        .filter(|j| j["paused"].as_bool() == Some(false))
        .count();
    
    // Calculate total resources committed
    let total_resources_committed: HashMap<String, f64> = site.construction_queue.all_jobs()
        .iter()
        .fold(HashMap::new(), |mut acc, job| {
            for (resource_id, amount) in &job.resources_committed {
                *acc.entry(resource_id.clone()).or_insert(0.0) += amount;
            }
            acc
        });

    // Build available resources map for build menu affordability checks
    let available_resources: HashMap<String, i64> = site.resources.iter()
        .map(|(rt, &amount)| (resource_type_to_string(*rt).to_string(), amount))
        .collect();

    let building_defs: Vec<serde_json::Value> = game_state.building_definitions.values()
        .map(|def| {
            let costs: Vec<serde_json::Value> = def.construction_costs.iter()
                .map(|c| serde_json::json!({
                    "resource_id": &c.resource_id,
                    "amount": c.amount,
                    "available": available_resources.get(&c.resource_id).copied().unwrap_or(0),
                    "can_afford": available_resources.get(&c.resource_id).copied().unwrap_or(0) >= c.amount as i64,
                }))
                .collect();
            let can_afford = costs.iter().all(|c| c["can_afford"].as_bool().unwrap_or(false));
            serde_json::json!({
                "id": &def.id,
                "name": &def.name,
                "category": format!("{:?}", def.category),
                "description": &def.description,
                "worker_capacity": def.worker_capacity,
                "power_consumption": def.power.as_ref().map(|p| p.base_mw).unwrap_or(0.0),
                "power_generation": def.power_output_mw.unwrap_or(0.0),
                "storage_capacity": def.storage_capacity.unwrap_or(0.0),
                "construction_time_ticks": def.construction_time_ticks,
                "construction_costs": costs,
                "can_afford": can_afford,
            })
        })
        .collect();
    let mut categories: Vec<String> = building_defs.iter()
        .filter_map(|d| d["category"].as_str().map(|s| s.to_string()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    categories.sort();

    let mut context = tera::Context::new();
    context.insert("site_id", &site_id.to_string());
    context.insert("construction_jobs", &construction_jobs);
    context.insert("active_count", &active_count);
    context.insert("total_resources_committed", &total_resources_committed);
    context.insert("building_definitions", &building_defs);
    context.insert("categories", &categories);
    
    let html = tmpl.render("components/_construction_tab.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Form data for starting construction
#[derive(Deserialize)]
pub struct BuildForm {
    pub building_def_id: String,
}

/// POST /site/{site_id}/build
///
/// Start construction of a new building
pub async fn start_construction(
    path: web::Path<String>,
    form: web::Form<BuildForm>,
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let site_id_str = path.into_inner();
    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);
    
    // Get mutable game state (this needs proper locking - will improve in next iteration)
    let mut game_state = simulation.get_state().await;
    
    // Create and execute command
    let command = StartConstruction {
        site_id,
        building_def_id: form.building_def_id.clone(),
    };
    
    let _events = command.execute(&mut game_state)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Cannot start construction: {}", e)))?;
    
    // TODO: Persist events to event store
    // For now, we just reload the construction tab
    
    // Return updated construction tab
    site_construction_tab(
        web::Path::from(site_id.to_string()),
        tmpl,
        simulation,
    ).await
}

/// POST /site/{site_id}/construction/{building_id}/cancel
///
/// Cancel a construction job
pub async fn cancel_construction(
    path: web::Path<(String, String)>,
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let (site_id_str, building_id_str) = path.into_inner();
    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);
    let building_id = BuildingId(Uuid::parse_str(&building_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid building ID: {}", e)))?);
    
    // Get mutable game state
    let mut game_state = simulation.get_state().await;
    
    // Create and execute command
    let command = CancelConstruction {
        site_id,
        building_id,
        waste_percentage: 0.2, // 20% waste
    };
    
    let _events = command.execute(&mut game_state)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Cannot cancel construction: {}", e)))?;
    
    // TODO: Persist events to event store
    
    // Return updated construction tab
    site_construction_tab(
        web::Path::from(site_id.to_string()),
        tmpl,
        simulation,
    ).await
}

/// POST /site/{site_id}/construction/{building_id}/pause
///
/// Toggle pause state of a construction job
pub async fn toggle_pause_construction(
    path: web::Path<(String, String)>,
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let (site_id_str, building_id_str) = path.into_inner();
    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);
    let building_id = BuildingId(Uuid::parse_str(&building_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid building ID: {}", e)))?);
    
    // Get mutable game state
    let mut game_state = simulation.get_state().await;
    
    // Find the job to check its current state
    let site = game_state.galaxy.find_site(&site_id)
        .ok_or_else(|| actix_web::error::ErrorNotFound(format!("Site {} not found", site_id)))?;
    
    let job = site.construction_queue.get(&building_id)
        .ok_or_else(|| actix_web::error::ErrorNotFound(format!("Construction job {} not found", building_id)))?;
    
    // Execute appropriate command based on current state
    let _events = if job.paused {
        let command = ResumeConstruction { site_id, building_id };
        command.execute(&mut game_state)
    } else {
        let command = PauseConstruction { site_id, building_id };
        command.execute(&mut game_state)
    };
    
    _events.map_err(|e| actix_web::error::ErrorBadRequest(format!("Cannot toggle pause: {}", e)))?;
    
    // TODO: Persist events to event store
    
    // Return updated construction tab
    site_construction_tab(
        web::Path::from(site_id.to_string()),
        tmpl,
        simulation,
    ).await
}


/// GET /site/{site_id}/resources
/// 
/// Render the resources tab showing stockpile, production/consumption rates, and storage utilization.
pub async fn site_resources_tab(
    path: web::Path<String>,
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let site_id_str = path.into_inner();
    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);
    
    // Get game state
    let game_state = simulation.get_state().await;
    let content = simulation.get_content().await;
    
    // Find the site
    let site = game_state.galaxy.find_site(&site_id)
        .ok_or_else(|| actix_web::error::ErrorNotFound(format!("Site {} not found", site_id)))?;
    
    // ── Storage capacity (real, aggregated from warehouse buildings) ──
    let site_storage = outpost_core::simulation::compute_site_storage(
        site, &game_state.building_definitions,
    );

    // ── Production / consumption rates (real, from active recipes) ──
    let rates = outpost_core::simulation::compute_resource_rates(
        site, &game_state.building_definitions, &content,
    );

    // ── Tooltip data: which buildings produce/consume each resource ──
    let producers = outpost_core::simulation::list_resource_producers(
        &game_state.building_definitions, &content,
    );
    let consumers = outpost_core::simulation::list_resource_consumers(
        &game_state.building_definitions, &content,
    );

    // ── Active production chains ──
    let chains_raw = outpost_core::simulation::active_production_chains(
        site, &game_state.building_definitions, &content,
    );

    // Build resource list
    let mut resources_list = Vec::new();
    for (resource_type, quantity) in site.resources.iter() {
        let resource_name = resource_type.name().to_string();
        let quantity_f64 = *quantity as f64;
        let resource_id = resource_type_to_string(*resource_type).to_string();

        // Rates
        let rate = rates.get(&resource_id).cloned().unwrap_or_default();
        let production_rate = rate.production;
        let consumption_rate = rate.consumption;
        let net_rate = rate.net();

        // Storage: per-resource share of site capacity
        let utilization_percent = (quantity_f64 / site_storage * 100.0).clamp(0.0, 100.0);
        let utilization_status = if utilization_percent >= 90.0 {
            "critical"
        } else if utilization_percent >= 70.0 {
            "warning"
        } else {
            "good"
        };

        let status = if *quantity <= 0 {
            "Depleted"
        } else if net_rate < -0.01 {
            "Deficit"
        } else if net_rate > 0.01 {
            "Surplus"
        } else {
            "Stable"
        };

        // Tooltip: producer and consumer building names
        let resource_producers = producers.get(&resource_id)
            .cloned()
            .unwrap_or_default();
        let resource_consumers = consumers.get(&resource_id)
            .cloned()
            .unwrap_or_default();

        // Icon mapping
        let icon = resource_icon(&resource_id);

        resources_list.push(serde_json::json!({
            "id": resource_id,
            "name": resource_name,
            "icon": icon,
            "quantity": quantity_f64,
            "capacity": site_storage,
            "utilization_percent": utilization_percent,
            "utilization_status": utilization_status,
            "production_rate": production_rate,
            "consumption_rate": consumption_rate,
            "net_rate": net_rate,
            "status": status,
            "producers": resource_producers,
            "consumers": resource_consumers,
        }));
    }
    
    // Sort: first by status priority (deficit/depleted first), then name
    resources_list.sort_by(|a, b| {
        let status_order = |s: &str| match s {
            "Depleted" => 0,
            "Deficit"  => 1,
            "Stable"   => 2,
            "Surplus"  => 3,
            _          => 4,
        };
        let sa = status_order(a["status"].as_str().unwrap_or(""));
        let sb = status_order(b["status"].as_str().unwrap_or(""));
        sa.cmp(&sb).then(
            a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
        )
    });
    
    // Storage summary
    let total_quantity: i64 = site.resources.iter().map(|(_, q)| q).sum();
    let utilization_pct = (total_quantity as f64 / site_storage * 100.0).clamp(0.0, 100.0);
    let storage_categories = vec![
        serde_json::json!({
            "name": "Total Storage",
            "used": total_quantity as f64,
            "capacity": site_storage,
            "utilization_percent": utilization_pct,
            "utilization_status": if utilization_pct >= 90.0 { "critical" }
                                   else if utilization_pct >= 70.0 { "warning" }
                                   else { "good" },
        }),
    ];

    // Active chains for the production chains panel
    let chains: Vec<serde_json::Value> = chains_raw.iter().map(|c| {
        let inputs: Vec<_> = c.inputs.iter()
            .map(|(k, v)| serde_json::json!({ "resource": k, "amount": v }))
            .collect();
        let outputs: Vec<_> = c.outputs.iter()
            .map(|(k, v)| serde_json::json!({ "resource": k, "amount": v }))
            .collect();
        serde_json::json!({
            "building_name": c.building_name,
            "recipe_name": c.recipe_name,
            "progress_pct": c.progress_pct,
            "inputs": inputs,
            "outputs": outputs,
        })
    }).collect();
    
    // Prepare context
    let mut context = tera::Context::new();
    context.insert("resources", &resources_list);
    context.insert("storage_categories", &storage_categories);
    context.insert("active_chains", &chains);
    context.insert("site_id", &site_id_str);
    
    // Render template
    let html = tmpl.render("components/resources_tab.html", &context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// POST /site/{site_id}/building/{bid}/demolish — remove a building instance
pub async fn demolish_building(
    path: web::Path<(String, String)>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let (site_id_str, bid_str) = path.into_inner();

    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);
    let building_id = BuildingId(Uuid::parse_str(&bid_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid building ID: {}", e)))?);

    simulation.with_state_mut(move |state| {
        for system in state.galaxy.systems.values_mut() {
            if let Some(site) = system.sites.get_mut(&site_id) {
                site.buildings.remove(&building_id);
                break;
            }
        }
    }).await;

    // Re-render buildings tab after demolish
    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", format!("/site/{}/buildings", site_id_str)))
        .finish())
}

/// Map a resource string ID to an emoji icon.
fn resource_icon(resource_id: &str) -> &'static str {
    match resource_id {
        "iron_ore"    => "⛏️",
        "iron_ingots" | "steel" => "🔩",
        "copper_ore" | "copper" => "🟠",
        "water"       => "💧",
        "food"        => "🌿",
        "oxygen"      => "💨",
        "electronics" => "⚡",
        "components"  => "⚙️",
        "silicon"     => "💎",
        "uranium_fuel"| "uranium" => "☢️",
        "ice"         => "🧊",
        "nutrients"   => "🧪",
        _             => "📦",
    }
}

/// Form data for admin set-resource debug action.
#[derive(Deserialize)]
pub struct SetResourceForm {
    pub resource_id: String,
    pub amount: f64,
}

/// POST /site/{id}/resources/set — debug: set a resource stockpile amount directly
pub async fn admin_set_resource(
    path: web::Path<String>,
    form: web::Form<SetResourceForm>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let site_id_str = path.into_inner();
    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);

    if let Ok(resource_type) = string_to_resource_type(&form.resource_id) {
        let amount = form.amount.max(0.0) as i64;
        simulation.with_state_mut(move |state| {
            for system in state.galaxy.systems.values_mut() {
                if let Some(site) = system.sites.get_mut(&site_id) {
                    site.resources.set(resource_type, amount);
                    break;
                }
            }
        }).await;
    }

    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", format!("/site/{}/resources", site_id_str)))
        .finish())
}

/// Form data for admin set-labor debug action.
#[derive(Deserialize)]
pub struct SetLaborForm {
    pub population: Option<u64>,
    pub morale: Option<f64>,
}

/// POST /site/{id}/labor/set — debug: set population and morale directly
pub async fn admin_set_labor(
    path: web::Path<String>,
    form: web::Form<SetLaborForm>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let site_id_str = path.into_inner();
    let site_id = SiteId(Uuid::parse_str(&site_id_str)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid site ID: {}", e)))?);

    let new_population = form.population;
    let new_morale = form.morale.map(|m| m.clamp(0.0, 100.0));
    simulation.with_state_mut(move |state| {
        for system in state.galaxy.systems.values_mut() {
            if let Some(site) = system.sites.get_mut(&site_id) {
                if let Some(pop) = new_population {
                    site.population.total = pop;
                }
                if let Some(mor) = new_morale {
                    site.population.morale = mor as f32;
                }
                break;
            }
        }
    }).await;

    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", format!("/site/{}/labor", site_id_str)))
        .finish())
}


