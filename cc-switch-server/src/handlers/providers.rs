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
    let _mutation = super::super::state::PROVIDER_MUTATIONS.lock().await;
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
    let _mutation = super::super::state::PROVIDER_MUTATIONS.lock().await;
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
    let _mutation = super::super::state::PROVIDER_MUTATIONS.lock().await;
    log::info!("[Providers] delete_provider requested id={}", id);

    let app_type: AppType = APP_TYPE.parse().unwrap_or(AppType::ClaudeCode);
    match cc_switch_lib::settings::get_effective_current_provider(&state.db, &app_type) {
        Ok(Some(current)) if current == id => return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "Select another direct provider before deleting this provider."})),
        ).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
        _ => {}
    }

    match state.db.delete_provider(&id, APP_TYPE) {
        Ok(()) => {
            log::info!("[Providers] delete_provider success id={}", id);
            Json(serde_json::json!({ "success": true })).into_response()
        }
        Err(cc_switch_lib::error::AppError::ProviderInUse(message)) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": message})),
        )
            .into_response(),
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
    let _mutation = super::super::state::PROVIDER_MUTATIONS.lock().await;
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

    let takeover_active = proxy_running;
    if !proxy_running && backup.is_some() && live_taken_over {
        return (StatusCode::CONFLICT, Json(serde_json::json!({"error": "The previous route was interrupted. Stop the local route to restore direct config before switching."}))).into_response();
    }
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
        log::info!("[Providers] switch_provider id={} path=direct_live", id);

        // (a) Capture the previous provider's env BEFORE backfill, then save the
        // current live config into the old provider's DB record. The captured env
        // is the reference for exclusive-key cleanup: an exclusive key the user
        // edited in the live file (value differs from the record) survives the
        // switch, while one left untouched is removed.
        let mut previous_env: Option<serde_json::Map<String, serde_json::Value>> = None;
        let app_type_enum: AppType = APP_TYPE.parse().unwrap_or(AppType::ClaudeCode);
        let current_id = match cc_switch_lib::settings::get_effective_current_provider(
            &state.db,
            &app_type_enum,
        ) {
            Ok(id) => id,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response()
            }
        };
        if let Some(ref old_id) = current_id {
            if old_id != &id {
                match state.db.get_provider(old_id, app_type) {
                    Ok(Some(mut old_provider)) => {
                        previous_env = old_provider
                            .settings_config
                            .get("env")
                            .and_then(|v| v.as_object())
                            .cloned();
                        match cc_switch_lib::live::backfill_current_live_config(
                            &old_provider.settings_config,
                        ) {
                            Ok(backfill_settings) => {
                                old_provider.settings_config = backfill_settings;
                                if let Err(e) = state.db.save_provider(app_type, &old_provider) {
                                    warnings.push(format!(
                                        "Backfill warning: failed to save old provider config: {}",
                                        e
                                    ));
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
                    Ok(None) => { /* old provider was deleted */ }
                    Err(e) => {
                        warnings.push(format!(
                            "Backfill warning: failed to read old provider: {}",
                            e
                        ));
                    }
                }
            }
        }

        // (b) Write direct live config only when route takeover is inactive.
        let live_settings = live::settings_for_live(&provider, state.proxy_listen_port, false);
        if let Err(e) =
            cc_switch_lib::live::apply_provider_to_live(&live_settings, previous_env.as_ref())
        {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::handlers::{proxy_start, proxy_stop};
    use crate::state::{TestHome, TEST_ENV_LOCK};
    use cc_switch_lib::database::Database;
    use cc_switch_lib::oauth::{codex::CodexOAuthManager, copilot::CopilotAuthManager};
    use serde_json::{json, Value};

    async fn body(response: impl IntoResponse) -> (StatusCode, Value) {
        let response = response.into_response();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn deletion_and_route_lifecycle_keep_direct_config_consistent() {
        let _env = TEST_ENV_LOCK.lock().await;
        let home = TestHome::new();
        let db = Arc::new(Database::init().unwrap());
        for id in ["direct", "route", "next"] {
            let provider: Provider = serde_json::from_value(json!({
                "id": id, "name": id,
                "settingsConfig": {"env": {"ANTHROPIC_BASE_URL": format!("https://{id}.example"), "ANTHROPIC_AUTH_TOKEN": id}},
                "meta": {"apiFormat": "anthropic"}
            })).unwrap();
            db.save_provider(APP_TYPE, &provider).unwrap();
        }
        db.set_current_provider("direct", APP_TYPE).unwrap();
        // Device selection takes precedence even if the DB flag diverges.
        cc_switch_lib::settings::set_current_provider(&AppType::ClaudeCode, Some("next")).unwrap();
        let occupied = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = occupied.local_addr().unwrap().port();
        let state = Arc::new(AppState {
            db: db.clone(),
            token: "test".into(),
            codex_oauth: Arc::new(CodexOAuthManager::new(home.path.clone())),
            copilot_oauth: Arc::new(CopilotAuthManager::new(home.path.clone())),
            proxy_server: Arc::new(tokio::sync::RwLock::new(None)),
            proxy_host: "127.0.0.1".into(),
            proxy_listen_port: port,
        });
        assert_eq!(
            body(delete_provider(State(state.clone()), Path("next".into())).await)
                .await
                .0,
            StatusCode::CONFLICT
        );
        cc_switch_lib::settings::set_current_provider(&AppType::ClaudeCode, None).unwrap();
        // DB fallback must supply the previous provider for switching/backfill.
        assert_eq!(
            body(switch_provider(State(state.clone()), Path("next".into())).await)
                .await
                .0,
            StatusCode::OK
        );
        db.set_proxy_target_provider_id("route").unwrap();
        assert_eq!(
            body(delete_provider(State(state.clone()), Path("route".into())).await)
                .await
                .0,
            StatusCode::CONFLICT
        );
        let path = live::get_live_settings_path();
        let before = std::fs::read(&path).unwrap();
        let (_, failed) = body(proxy_start(State(state.clone())).await).await;
        assert_eq!(failed["success"], false);
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(db.get_live_backup(APP_TYPE).unwrap().is_none());
        drop(occupied);
        let (_, started) = body(proxy_start(State(state.clone())).await).await;
        assert_eq!(started["success"], true, "{started}");
        let takeover = std::fs::read(&path).unwrap();
        assert_eq!(
            body(switch_provider(State(state.clone()), Path("direct".into())).await)
                .await
                .0,
            StatusCode::OK
        );
        assert_eq!(std::fs::read(&path).unwrap(), takeover);
        assert_eq!(
            db.get_proxy_target_provider_id().unwrap().as_deref(),
            Some("route")
        );
        cc_switch_lib::config::update_json_file(&path, |value| {
            value["env"]["API_TIMEOUT_MS"] = json!("12345");
            value["permissions"] = json!({"allow": ["Read"]});
            Ok(())
        })
        .unwrap();
        // Simulate a server restart: only the persisted takeover snapshot remains.
        let server = state.proxy_server.write().await.take().unwrap();
        server.stop().await.unwrap();
        let (_, status) =
            body(crate::proxy::handlers::proxy_status(State(state.clone())).await).await;
        assert_eq!(status["recovery_required"], true);
        let (_, stopped) = body(proxy_stop(State(state.clone())).await).await;
        assert_eq!(stopped["success"], true, "{stopped}");
        let restored: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(restored["env"]["ANTHROPIC_AUTH_TOKEN"], "direct");
        assert_eq!(restored["env"]["API_TIMEOUT_MS"], "12345");
        assert_eq!(restored["permissions"]["allow"], json!(["Read"]));
        assert!(db.get_live_backup(APP_TYPE).unwrap().is_none());
        let before = std::fs::read(&path).unwrap();
        body(proxy_stop(State(state.clone())).await).await;
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::write(&path, "invalid JSON").unwrap();
        let (_, failed) = body(proxy_start(State(state.clone())).await).await;
        assert_eq!(failed["success"], false);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "invalid JSON");
        assert!(state.proxy_server.read().await.is_none());
        assert!(db.get_live_backup(APP_TYPE).unwrap().is_none());
    }
}
