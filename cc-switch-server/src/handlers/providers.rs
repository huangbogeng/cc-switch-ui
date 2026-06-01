//! Provider handlers

use super::super::state::AppState;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use cc_switch_lib::database::Provider;
use cc_switch_lib::live;
use cc_switch_lib::providers::AppType;
use std::sync::Arc;

const APP_TYPE: &str = cc_switch_lib::DEFAULT_APP_TYPE;

pub async fn list_providers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::debug!("[Providers] list_providers requested app_type={}", APP_TYPE);
    match state.db.list_providers(APP_TYPE) {
        Ok(providers) => {
            log::debug!(
                "[Providers] list_providers success app_type={} count={}",
                APP_TYPE,
                providers.len()
            );
            Json(serde_json::json!({ "providers": providers })).into_response()
        }
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
    let app_type_enum: AppType = APP_TYPE.parse().unwrap_or(AppType::ClaudeCode);
    log::debug!(
        "[Providers] get_current_provider requested app_type={}",
        APP_TYPE
    );
    match cc_switch_lib::settings::get_effective_current_provider(&state.db, &app_type_enum) {
        Ok(id) => {
            log::debug!(
                "[Providers] get_current_provider success app_type={} current_provider_id={:?}",
                APP_TYPE,
                id
            );
            Json(serde_json::json!({ "current_provider_id": id })).into_response()
        }
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
    log::info!("[Providers] get_provider requested id={}", id);
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
    Json(mut provider): Json<Provider>,
) -> impl IntoResponse {
    log::info!(
        "[Providers] save_provider requested id={} name={}",
        provider.id,
        provider.name
    );
    // Normalise the key field so the DB record always matches the
    // declared apiKeyField.  Prevents stale ANTHROPIC_API_KEY values
    // from persisting across edits and surfacing during route toggles.
    cc_switch_lib::providers::normalize_provider_schema(&mut provider);
    match state.db.save_provider(APP_TYPE, &provider) {
        Ok(()) => {
            log::info!("[Providers] save_provider success id={}", provider.id);
            Json(serde_json::json!({ "success": true })).into_response()
        }
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
    Json(mut provider): Json<Provider>,
) -> impl IntoResponse {
    log::info!(
        "[Providers] update_provider requested path_id={} payload_id={}",
        id,
        provider.id
    );
    if provider.id != id {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Provider ID mismatch"})),
        )
            .into_response();
    }
    cc_switch_lib::providers::normalize_provider_schema(&mut provider);
    match state.db.save_provider(APP_TYPE, &provider) {
        Ok(()) => {
            log::info!("[Providers] update_provider success id={}", provider.id);
            Json(serde_json::json!({ "success": true })).into_response()
        }
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
    log::info!("[Providers] delete_provider requested id={}", id);

    // Clear stale references before deleting so the deleted provider
    // doesn't leave dangling current/target pointers.
    if let Ok(Some(current_id)) = state.db.get_current_provider_id(APP_TYPE) {
        if current_id == id {
            let _ = state.db.set_current_provider("", APP_TYPE);
        }
    }
    if let Ok(Some(target_id)) = state.db.get_proxy_target_provider_id() {
        if target_id == id {
            let _ = state.db.set_proxy_target_provider_id("");
        }
    }

    match state.db.delete_provider(&id, APP_TYPE) {
        Ok(()) => {
            log::info!("[Providers] delete_provider success id={}", id);
            Json(serde_json::json!({ "success": true })).into_response()
        }
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
    log::info!("[Providers] switch_provider requested id={}", id);

    let app_type = APP_TYPE;

    // 1. Get provider
    let provider = match state.db.get_provider(&id, app_type) {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Provider not found"})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    // 2. Determine takeover state
    let backup = state.db.get_live_backup(app_type).ok().flatten();
    let live_taken_over = cc_switch_lib::live::detect_takeover_in_live_config();
    let proxy_running = {
        let guard = state.proxy_server.read().await;
        match guard.as_ref() {
            Some(server) => server.is_running().await,
            None => false,
        }
    };

    let takeover_active = (backup.is_some() || live_taken_over) && proxy_running;
    let mut warnings: Vec<String> = Vec::new();

    // Always update current-provider state. During route takeover we intentionally
    // avoid changing route target or rewriting live config; those belong to the
    // explicit route-target and route-lifecycle actions.
    if takeover_active {
        log::info!(
            "[Providers] switch_provider id={} path=takeover_select_only",
            id
        );
    } else {
        // Direct-live path: backfill current live settings, then apply selected provider.
        log::info!(
            "[Providers] switch_provider id={} path=direct_live",
            id
        );

        // (a) Backfill: save current live config to old provider's DB record
        let app_type_enum: AppType = APP_TYPE.parse().unwrap_or(AppType::ClaudeCode);
        let current_id = cc_switch_lib::settings::get_current_provider(&app_type_enum);
        if let Some(ref old_id) = current_id {
            if old_id != &id {
                match cc_switch_lib::live::backfill_current_live_config() {
                    Ok(backfill_settings) => {
                        match state.db.get_provider(old_id, app_type) {
                            Ok(Some(mut old_provider)) => {
                                old_provider.settings_config = backfill_settings;
                                if let Err(e) =
                                    state.db.save_provider(app_type, &old_provider)
                                {
                                    warnings.push(format!(
                                        "Backfill warning: failed to save old provider config: {}",
                                        e
                                    ));
                                }
                            }
                            Ok(None) => { /* old provider was deleted */ }
                            Err(e) => {
                                warnings.push(format!(
                                    "Backfill warning: failed to read old provider: {}",
                                    e
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        warnings.push(format!(
                            "Backfill warning: failed to read live config: {}",
                            e
                        ));
                    }
                }
            }
        }

        // (b) Write direct live config only when route takeover is inactive.
        let live_settings = live::settings_for_live(&provider, state.proxy_listen_port, false);
        if let Err(e) = cc_switch_lib::live::apply_provider_to_live(&live_settings) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to apply config: {}", e)
                })),
            )
                .into_response();
        }
    }

    let app_type_enum: AppType = APP_TYPE.parse().unwrap_or(AppType::ClaudeCode);
    if let Err(e) = cc_switch_lib::settings::set_current_provider(&app_type_enum, Some(&id)) {
        warnings.push(format!("Failed to save device setting: {}", e));
    }
    if let Err(e) = state.db.set_current_provider(&id, app_type) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to set current provider: {}", e)
            })),
        )
            .into_response();
    }

    log::info!(
        "[Providers] switch_provider success id={} warnings={}",
        id,
        warnings.len()
    );

    Json(serde_json::json!({
        "success": true,
        "warnings": warnings
    }))
    .into_response()
}
