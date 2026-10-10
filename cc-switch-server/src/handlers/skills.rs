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

static MUTATIONS: std::sync::Mutex<()> = std::sync::Mutex::new(());

const APP_TYPE: &str = "claude_code";

pub async fn list_skills(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.get_all_skills(APP_TYPE) {
        Ok(skills) => Json(json!({ "skills": skills })).into_response(),
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
    super::file_mutation(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        let mut skill = body;
        skill.app_type = APP_TYPE.to_string();
        log::info!("[Skills] save_skill id={} name={}", skill.id, skill.name);
        if let Err(e) = state
            .db
            .validate_skill(&skill)
            .and_then(|()| cc_switch_lib::skills::ensure_skill_ssot(&skill))
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
                .into_response();
        }
        match state.db.save_skill(&skill) {
            Ok(()) => match cc_switch_lib::skills::sync_enabled_to_claude(&state.db, APP_TYPE) {
                Ok(()) => Json(json!({"success": true})).into_response(),
                Err(e) => super::sync_failure("Skills", &e),
            },
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response(),
        }
    })
    .await
}

pub async fn delete_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    super::file_mutation(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        log::info!("[Skills] delete_skill id={}", id);
        let skill = match state.db.get_all_skills(APP_TYPE) {
            Ok(skills) => skills.into_iter().find(|skill| skill.id == id),
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": e.to_string()})),
                )
                    .into_response()
            }
        };
        let Some(skill) = skill else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Skill not found"})),
            )
                .into_response();
        };
        if let Err(e) = cc_switch_lib::skills::remove_from_claude(&skill.directory) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response();
        }
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
    })
    .await
}

pub async fn sync_skills(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[Skills] Manual sync requested");
    let state = state.clone();
    match tokio::task::spawn_blocking(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        cc_switch_lib::skills::sync_enabled_to_claude(&state.db, APP_TYPE)
    })
    .await
    {
        Ok(Ok(())) => Json(json!({ "success": true })).into_response(),
        Ok(Err(e)) => {
            log::error!("Failed to sync skills: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
        Err(e) => {
            log::error!("Skills sync task panicked: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Internal error"})),
            )
                .into_response()
        }
    }
}

pub async fn import_skills(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[Skills] Import from Claude Code skills/plugins requested");
    let state = state.clone();
    match tokio::task::spawn_blocking(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        cc_switch_lib::skills::import_from_claude(&state.db, APP_TYPE)
    })
    .await
    {
        Ok(Ok(count)) => Json(json!({ "success": true, "imported": count })).into_response(),
        Ok(Err(e)) => {
            log::error!("Failed to import skills: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
        Err(e) => {
            log::error!("Skills import task panicked: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Internal error"})),
            )
                .into_response()
        }
    }
}

pub async fn toggle_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    super::file_mutation(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        log::info!("[Skills] Toggle enabled for id={}", id);
        match state.db.get_all_skills(APP_TYPE) {
            Ok(skills) => {
                if let Some(skill) = skills.into_iter().find(|s| s.id == id) {
                    let mut toggled = skill.clone();
                    toggled.enabled = !toggled.enabled;
                    match state.db.save_skill(&toggled) {
                        Ok(()) => {
                            match cc_switch_lib::skills::sync_enabled_to_claude(&state.db, APP_TYPE)
                            {
                                Ok(()) => {
                                    Json(json!({ "success": true, "enabled": toggled.enabled }))
                                        .into_response()
                                }
                                Err(e) => super::sync_failure("Skills", &e),
                            }
                        }
                        Err(e) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({"error": e.to_string()})),
                        )
                            .into_response(),
                    }
                } else {
                    (
                        StatusCode::NOT_FOUND,
                        Json(json!({"error": "Skill not found"})),
                    )
                        .into_response()
                }
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response(),
        }
    })
    .await
}
