//! Provider handlers

use super::super::state::AppState;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use cc_switch_lib::database::Provider;
use serde_json::{json, Value};
use std::sync::Arc;

const APP_TYPE: &str = "claude";
const PROXY_TOKEN_PLACEHOLDER: &str = "PROXY_MANAGED";

pub async fn list_providers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.list_providers(APP_TYPE) {
        Ok(providers) => Json(serde_json::json!({ "providers": providers })).into_response(),
        Err(e) => {
            log::error!("Failed to list providers: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn get_current_provider(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.get_current_provider_id(APP_TYPE) {
        Ok(id) => Json(serde_json::json!({ "current_provider_id": id })).into_response(),
        Err(e) => {
            log::error!("Failed to get current provider: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn get_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_provider(&id, APP_TYPE) {
        Ok(Some(provider)) => Json(serde_json::json!({ "provider": provider })).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Provider not found"})),
        )
            .into_response(),
        Err(e) => {
            log::error!("Failed to get provider: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn save_provider(
    State(state): State<Arc<AppState>>,
    Json(provider): Json<Provider>,
) -> impl IntoResponse {
    match state.db.save_provider(APP_TYPE, &provider) {
        Ok(()) => Json(serde_json::json!({ "success": true })).into_response(),
        Err(e) => {
            log::error!("Failed to save provider: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(provider): Json<Provider>,
) -> impl IntoResponse {
    if provider.id != id {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Provider ID mismatch"})),
        )
            .into_response();
    }
    match state.db.save_provider(APP_TYPE, &provider) {
        Ok(()) => Json(serde_json::json!({ "success": true })).into_response(),
        Err(e) => {
            log::error!("Failed to update provider: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.delete_provider(&id, APP_TYPE) {
        Ok(()) => Json(serde_json::json!({ "success": true })).into_response(),
        Err(e) => {
            log::error!("Failed to delete provider: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn switch_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Get provider
    let provider_result = state.db.get_provider(&id, APP_TYPE);
    let provider = match provider_result {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Provider not found"})),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    // Apply to live config. Codex OAuth providers must point Claude Code at
    // the local proxy; the proxy then injects real OAuth credentials upstream.
    let live_settings = settings_for_live(&provider, state.proxy_listen_port);
    let apply_result = cc_switch_lib::live::apply_provider_to_live(&live_settings);
    if let Err(err) = apply_result {
        log::error!("Failed to apply provider to live config: {}", err);
        let err_msg = format!("Failed to apply config: {}", err);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": err_msg})),
        )
            .into_response();
    }

    // Update current provider in database
    match state.db.set_current_provider(&id, APP_TYPE) {
        Ok(()) => Json(serde_json::json!({ "success": true })).into_response(),
        Err(e) => {
            log::error!("Failed to switch provider: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

fn settings_for_live(provider: &Provider, proxy_port: u16) -> Value {
    let mut settings = provider.settings_config.clone();
    if !is_codex_oauth_provider(provider) {
        return settings;
    }

    if !settings.is_object() {
        settings = json!({});
    }
    let root = settings
        .as_object_mut()
        .expect("settings should be normalized to object");
    let env = root.entry("env".to_string()).or_insert_with(|| json!({}));
    if !env.is_object() {
        *env = json!({});
    }
    let env = env
        .as_object_mut()
        .expect("settings env should be normalized to object");
    env.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        json!(format!("http://127.0.0.1:{}", proxy_port)),
    );
    env.insert(
        "ANTHROPIC_AUTH_TOKEN".to_string(),
        json!(PROXY_TOKEN_PLACEHOLDER),
    );
    settings
}

fn is_codex_oauth_provider(provider: &Provider) -> bool {
    provider.meta.get("providerType").and_then(Value::as_str) == Some("codex_oauth")
}
