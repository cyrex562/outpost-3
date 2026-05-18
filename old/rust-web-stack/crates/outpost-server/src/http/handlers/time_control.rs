//! Time control API handlers for pause/resume/speed control.

use actix_web::{web, HttpResponse, Result};
use outpost_core::simulation::TimeCommand;
use serde::{Deserialize, Serialize};
use crate::services::SimulationService;

/// Request body for setting simulation speed
#[derive(Serialize, Deserialize)]
pub struct SetSpeedRequest {
    pub multiplier: u32,
}

/// Response for time status endpoint
#[derive(Serialize, Deserialize)]
pub struct TimeStatusResponse {
    pub current_tick: u64,
    pub game_time: String,
    pub paused: bool,
    pub speed_multiplier: u32,
}

/// Get current time status
///
/// GET /api/time/status
pub async fn get_time_status(
    service: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let status = TimeStatusResponse {
        current_tick: service.current_tick().await,
        game_time: service.game_time().await,
        paused: service.is_paused().await,
        speed_multiplier: service.speed_multiplier().await,
    };

    Ok(HttpResponse::Ok().json(status))
}

/// Pause the simulation
///
/// POST /api/time/pause
pub async fn pause_simulation(
    service: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    service
        .execute_time_command(TimeCommand::Pause)
        .await
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "paused",
        "current_tick": service.current_tick().await,
    })))
}

/// Resume the simulation
///
/// POST /api/time/resume
pub async fn resume_simulation(
    service: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    service
        .execute_time_command(TimeCommand::Resume)
        .await
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "running",
        "current_tick": service.current_tick().await,
    })))
}

/// Toggle pause state
///
/// POST /api/time/toggle
pub async fn toggle_pause(
    service: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    service
        .execute_time_command(TimeCommand::TogglePause)
        .await
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    let paused = service.is_paused().await;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "paused": paused,
        "status": if paused { "paused" } else { "running" },
        "current_tick": service.current_tick().await,
    })))
}

/// Set simulation speed multiplier
///
/// POST /api/time/speed
/// Body: { "multiplier": 5 }
pub async fn set_speed(
    service: web::Data<SimulationService>,
    req: web::Json<SetSpeedRequest>,
) -> Result<HttpResponse> {
    service
        .execute_time_command(TimeCommand::SetSpeed(req.multiplier))
        .await
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "speed_multiplier": req.multiplier,
        "current_tick": service.current_tick().await,
    })))
}

/// Enable idle safety (auto-pause on critical resource depletion)
///
/// POST /api/time/idle-safety/enable
pub async fn enable_idle_safety(
    service: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    service
        .execute_time_command(TimeCommand::EnableIdleSafety)
        .await
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "idle_safety_enabled": true,
    })))
}

/// Disable idle safety
///
/// POST /api/time/idle-safety/disable
pub async fn disable_idle_safety(
    service: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    service
        .execute_time_command(TimeCommand::DisableIdleSafety)
        .await
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "idle_safety_enabled": false,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use outpost_core::domain::GameState;

    #[actix_web::test]
    async fn test_get_time_status() {
        let state = GameState::new("Test".to_string(), 42);
        let content = outpost_core::content::ContentLoader::new();
        let service = web::Data::new(SimulationService::new(state, content));

        let app = test::init_service(
            App::new()
                .app_data(service.clone())
                .route("/api/time/status", web::get().to(get_time_status)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/time/status")
            .to_request();

        let resp: TimeStatusResponse = test::call_and_read_body_json(&app, req).await;

        assert_eq!(resp.current_tick, 0);
        assert_eq!(resp.paused, false);
        assert_eq!(resp.speed_multiplier, 1);
    }

    #[actix_web::test]
    async fn test_pause_simulation() {
        let state = GameState::new("Test".to_string(), 42);
        let content = outpost_core::content::ContentLoader::new();
        let service = web::Data::new(SimulationService::new(state, content));

        let app = test::init_service(
            App::new()
                .app_data(service.clone())
                .route("/api/time/pause", web::post().to(pause_simulation)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/time/pause")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        assert!(service.is_paused().await);
    }

    #[actix_web::test]
    async fn test_set_speed() {
        let state = GameState::new("Test".to_string(), 42);
        let content = outpost_core::content::ContentLoader::new();
        let service = web::Data::new(SimulationService::new(state, content));

        let app = test::init_service(
            App::new()
                .app_data(service.clone())
                .route("/api/time/speed", web::post().to(set_speed)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/time/speed")
            .set_json(SetSpeedRequest { multiplier: 5 })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        assert_eq!(service.speed_multiplier().await, 5);
    }
}
