use actix_web::web;

use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .route("/", web::get().to(handlers::index))
            .route("/dashboard", web::get().to(handlers::dashboard))
            // Colonies overview (V5)
            .route("/colonies", web::get().to(handlers::colonies_handler::colonies_list))
            // Site detail routes (V5)
            .route("/site/{id}", web::get().to(handlers::site_handlers::site_detail))
            .route("/site/{id}/buildings", web::get().to(handlers::site_handlers::site_buildings_tab))
            .route("/site/{id}/construction", web::get().to(handlers::site_handlers::site_construction_tab))
            .route("/site/{id}/resources", web::get().to(handlers::site_handlers::site_resources_tab))
            .route("/site/{id}/build", web::post().to(handlers::site_handlers::start_construction))
            .route("/site/{id}/construction/{job_id}/cancel", web::post().to(handlers::site_handlers::cancel_construction))
            .route("/site/{id}/construction/{job_id}/pause", web::post().to(handlers::site_handlers::toggle_pause_construction))
            // Legacy routes temporarily disabled during V5 migration
            // .route("/colony/{id}", web::get().to(handlers::view_colony))
            // .route("/colony/create", web::post().to(handlers::create_colony))
            // .route("/colony/{id}/building", web::post().to(handlers::construct_building))
            // .route("/colony/{colony_id}/building/{building_id}/upgrade", web::post().to(handlers::upgrade_building))
            // .route("/colony/{colony_id}/building/{building_id}/repair", web::post().to(handlers::colony_actions::repair_building))
            // .route("/colony/{colony_id}/building/{building_id}/toggle", web::post().to(handlers::colony_actions::toggle_building))
            // .route("/colony/{id}/building/{building_id}/allocate_labor", web::post().to(handlers::colony_actions::allocate_labor))
            // .route("/colony/{id}/building/{building_id}/deallocate_labor", web::post().to(handlers::colony_actions::deallocate_labor))
            // .route("/colony/{id}/turn", web::post().to(handlers::advance_turn))
            .route("/map", web::get().to(handlers::view_starmap))
            // Time control API routes
            .route("/api/time/status", web::get().to(handlers::time_control::get_time_status))
            .route("/api/time/pause", web::post().to(handlers::time_control::pause_simulation))
            .route("/api/time/resume", web::post().to(handlers::time_control::resume_simulation))
            .route("/api/time/toggle", web::post().to(handlers::time_control::toggle_pause))
            .route("/api/time/speed", web::post().to(handlers::time_control::set_speed))
            .route("/api/time/idle-safety/enable", web::post().to(handlers::time_control::enable_idle_safety))
            .route("/api/time/idle-safety/disable", web::post().to(handlers::time_control::disable_idle_safety))
    );
}
