//! Difficulty settings endpoints (`GET /api/difficulty`, `PUT /api/difficulty/:key`).
//!
//! Player-facing, not gated by `admin_mode`. Backed by the same session actor
//! (same WorldDatabase) as the admin routes. On `PUT player_hp`, the active
//! character's effective max HP is recomputed and persisted.

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;

use harsh_core::character_recalc::recalc_hp_for_difficulty;
use harsh_core::difficulty::DifficultySettingsResponse;
use harsh_core::repositories::difficulty::DifficultySettingsRepository;
use harsh_core::repositories::entity::EntityRepository;

use crate::state::AppState;

/// Difficulty settings routes, merged into the main router before `with_state`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/difficulty", get(get_difficulty))
        .route("/api/difficulty/:key", put(set_difficulty_grade))
}

fn respond(result: Result<Value, String>) -> Response {
    crate::response::respond_with(result, "DifficultyError")
}

/// `GET /api/difficulty` — return all dimensions with current grades.
async fn get_difficulty(State(state): State<AppState>) -> Response {
    let result = state
        .session
        .read(|db| {
            let repo = DifficultySettingsRepository::new(db);
            let dimensions = repo.list_dimensions()?;
            let resp = DifficultySettingsResponse { dimensions };
            serde_json::to_value(resp).map_err(|e| e.to_string())
        })
        .await;
    respond(result)
}

/// Request body for `PUT /api/difficulty/:key`.
#[derive(Debug, Deserialize)]
struct SetGradeRequest {
    grade: i32,
}

/// `PUT /api/difficulty/:key` — write a grade; if key is `player_hp`, recalculate HP.
async fn set_difficulty_grade(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(body): Json<SetGradeRequest>,
) -> Response {
    let result = state
        .session
        .read(move |db| {
            let repo = DifficultySettingsRepository::new(db);
            // Validate + persist.
            repo.set_grade(&key, body.grade)?;

            // For player_hp: recompute and persist the active character's HP.
            if key == "player_hp" {
                let profile = repo.load_profile()?;
                let entity_repo = EntityRepository::new(db);
                if let Some(mut character) = entity_repo.load_first_character()? {
                    recalc_hp_for_difficulty(&mut character, profile.player_hp_mult);
                    entity_repo.save_character(&character, false, true, false)?;
                }
            }

            // Return the updated dimension.
            let dim = repo.get_dimension(&key)?;
            serde_json::to_value(dim).map_err(|e| e.to_string())
        })
        .await;
    respond(result)
}

// ---------------------------------------------------------------------------
// Route-level tests (API shape)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_grade_request_deserializes() {
        let json = r#"{"grade": 2}"#;
        let req: SetGradeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.grade, 2);
    }
}
