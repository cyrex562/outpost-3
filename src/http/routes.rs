use actix_web::web;

use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .route("/", web::get().to(handlers::index))
            .route("/colony/{id}", web::get().to(handlers::view_colony))
            .route("/colony/create", web::post().to(handlers::create_colony))
            .route("/colony/{id}/building", web::post().to(handlers::construct_building))
            .route("/colony/{colony_id}/building/{building_id}/upgrade", web::post().to(handlers::upgrade_building))
            .route("/colony/{colony_id}/building/{building_id}/repair", web::post().to(handlers::repair_building))
            .route("/colony/{id}/turn", web::post().to(handlers::advance_turn))
    );
}
