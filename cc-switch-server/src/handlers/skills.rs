//! Skills management handlers

use super::super::state::AppState;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use cc_switch_lib::database::SkillRecord;
use serde_json::json;
use std::sync::Arc;

const APP_TYPE: &str = "claude_code";

pub async fn list_skills(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.get_all_skills(APP_TYPE) {
        Ok(skills) => {
            Json(json!({ "skills": skills })).into_response()
        }
        Err(e) => {
            log::error!("Failed to list skills: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn save_skill(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SkillRecord>,
) -> impl IntoResponse {
    let mut skill = body;
    skill.app_type = APP_TYPE.to_string();
    skill.enabled = true; // Always enabled for now

    log::info!("[Skills] save_skill id={} name={}", skill.id, skill.name);
    match state.db.save_skill(&skill) {
        Ok(()) => {
            // Create SSOT directory and SKILL.md so future syncs can find the files
            if let Err(e) = cc_switch_lib::skills::ensure_skill_ssot(&skill) {
                log::warn!("[Skills] Failed to create SSOT for '{}': {}", skill.directory, e);
            }
            Json(json!({ "success": true })).into_response()
        }
        Err(e) => {
            log::error!("Failed to save skill: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn delete_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    log::info!("[Skills] delete_skill id={}", id);
    match state.db.delete_skill(&id) {
        Ok(true) => Json(json!({ "success": true })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Skill not found"})),
        )
            .into_response(),
        Err(e) => {
            log::error!("Failed to delete skill: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn sync_skills(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[Skills] Manual sync requested");
    match cc_switch_lib::skills::sync_enabled_to_claude(&state.db, APP_TYPE) {
        Ok(()) => Json(json!({ "success": true })).into_response(),
        Err(e) => {
            log::error!("Failed to sync skills: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn import_skills(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[Skills] Import from Claude Code skills/plugins requested");
    match cc_switch_lib::skills::import_from_claude(&state.db, APP_TYPE) {
        Ok(count) => Json(json!({ "success": true, "imported": count })).into_response(),
        Err(e) => {
            log::error!("Failed to import skills: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn toggle_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    log::info!("[Skills] Toggle enabled for id={}", id);
    match state.db.get_all_skills(APP_TYPE) {
        Ok(skills) => {
            if let Some(skill) = skills.into_iter().find(|s| s.id == id) {
                let mut toggled = skill.clone();
                toggled.enabled = !toggled.enabled;
                match state.db.save_skill(&toggled) {
                    Ok(()) => Json(json!({ "success": true, "enabled": toggled.enabled })).into_response(),
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": e.to_string()})),
                    ).into_response(),
                }
            } else {
                (StatusCode::NOT_FOUND, Json(json!({"error": "Skill not found"}))).into_response()
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ).into_response(),
    }
}
