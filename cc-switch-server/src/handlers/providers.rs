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
            Json(serde_json::json!({ "current_provider_id": id, "recovery_required": cc_switch_lib::provider_operations::recovery_pending() })).into_response()
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

#[derive(serde::Deserialize)]
pub struct ProviderSaveRequest {
    #[serde(flatten)]
    provider: Provider,
    #[serde(default)]
    base: Option<Provider>,
    #[serde(default)]
    create: bool,
}

pub async fn save_provider(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProviderSaveRequest>,
) -> impl IntoResponse {
    persist_provider(state, request).await
}

pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<ProviderSaveRequest>,
) -> impl IntoResponse {
    if request.provider.id != id {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Provider ID mismatch"})),
        )
            .into_response();
    }
    persist_provider(state, request).await
}

async fn persist_provider(
    state: Arc<AppState>,
    request: ProviderSaveRequest,
) -> axum::response::Response {
    let _mutation = super::super::state::PROVIDER_MUTATIONS.lock().await;
    let route_active = state.proxy_server.read().await.is_some();
    let result =
        tokio::task::spawn_blocking(move || persist_provider_record(&state, request, route_active))
            .await;
    match result {
        Ok(Ok(result)) => Json(result).into_response(),
        Ok(Err(error)) => operation_error(error),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

fn persist_provider_record(
    state: &AppState,
    mut request: ProviderSaveRequest,
    route_active: bool,
) -> Result<serde_json::Value, cc_switch_lib::error::AppError> {
    use cc_switch_lib::{error::AppError, provider_operations};
    let warnings = provider_operations::recover(&state.db)?;
    cc_switch_lib::providers::normalize_provider_schema(&mut request.provider);
    let old = state.db.get_provider(&request.provider.id, APP_TYPE)?;
    if request.create && old.is_some() {
        return Err(AppError::Conflict {
            path: "provider ID already exists; edit the existing record or choose another ID"
                .into(),
        });
    }
    if let Some(base) = &request.base {
        if old
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|source| AppError::JsonSerialize { source })?
            != Some(
                serde_json::to_value(base).map_err(|source| AppError::JsonSerialize { source })?,
            )
            && old
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(|source| AppError::JsonSerialize { source })?
                != Some(
                    serde_json::to_value(&request.provider)
                        .map_err(|source| AppError::JsonSerialize { source })?,
                )
        {
            return Err(AppError::Conflict {
                path: "provider record; reopen the editor to load the latest version".into(),
            });
        }
    }
    let provider = &mut request.provider;
    if provider.id.trim().is_empty()
        || provider.name.trim().is_empty()
        || !provider.settings_config.is_object()
        || provider
            .settings_config
            .get("env")
            .is_some_and(|env| !env.is_object())
    {
        return Err(AppError::InvalidInput(
            "Provider ID, name and an object configuration are required".into(),
        ));
    }
    let current =
        cc_switch_lib::settings::get_effective_current_provider(&state.db, &AppType::ClaudeCode)?;
    let applied = current.as_deref() == Some(&provider.id) && !route_active;
    if applied {
        if state.db.get_live_backup(APP_TYPE)?.is_some() {
            return Err(AppError::ProviderInUse(
                "Restore the interrupted local route before editing the direct provider".into(),
            ));
        }
        let desired = live::settings_for_live(provider, state.proxy_listen_port, false);
        let base = old
            .as_ref()
            .map(|p| live::settings_for_live(p, state.proxy_listen_port, false))
            .unwrap_or_else(|| serde_json::json!({}));
        provider_operations::apply(&state.db, Some(provider), &desired, &base)?;
    } else {
        state.db.save_provider(APP_TYPE, provider)?;
    }
    Ok(
        serde_json::json!({"success": true, "saved": true, "applied": applied, "warnings": warnings,
            "message": if applied { "Saved and applied to direct config." } else if route_active { "Saved. The local route remains active. The selected direct provider is applied when the route stops." } else { "Saved. Select this provider to apply it." }}),
    )
}

pub async fn recover_provider_operation(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let _mutation = super::super::state::PROVIDER_MUTATIONS.lock().await;
    match tokio::task::spawn_blocking(move || cc_switch_lib::provider_operations::recover(&state.db)).await {
        Ok(Ok(warnings)) => Json(serde_json::json!({"success": true, "warnings": warnings, "message": "Unfinished provider operation recovered. Review the current provider before editing."})).into_response(),
        Ok(Err(error)) => operation_error(error),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": error.to_string()}))).into_response(),
    }
}

fn operation_error(error: cc_switch_lib::error::AppError) -> axum::response::Response {
    use cc_switch_lib::error::AppError;
    let status = match &error {
        AppError::Conflict { .. } | AppError::ProviderInUse(_) => StatusCode::CONFLICT,
        AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let pending = cc_switch_lib::provider_operations::recovery_pending();
    let message = if pending {
        format!("{error}. An unfinished provider operation is pending; use Recover operation before switching.")
    } else {
        error.to_string()
    };
    (
        status,
        Json(serde_json::json!({"error": message, "recovery_required": pending})),
    )
        .into_response()
}

pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let _mutation = super::super::state::PROVIDER_MUTATIONS.lock().await;
    if let Err(error) = cc_switch_lib::provider_operations::recover(&state.db) {
        return operation_error(error);
    }

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
    let route_active = {
        let guard = state.proxy_server.read().await;
        match guard.as_ref() {
            Some(server) => server.is_running().await,
            None => false,
        }
    };
    let result = tokio::task::spawn_blocking(move || -> Result<Option<serde_json::Value>, cc_switch_lib::error::AppError> {
        let warnings = cc_switch_lib::provider_operations::recover(&state.db)?;
        if state.db.get_provider(&id, APP_TYPE)?.is_none() {
            return Ok(None);
        }
        let applied = cc_switch_lib::provider_operations::switch(&state.db, &id, state.proxy_listen_port, route_active)?;
        Ok(Some(serde_json::json!({"success": true, "saved": true, "applied": applied, "warnings": warnings,
            "message": if route_active { "Direct provider selected. It will be applied when the local route stops." } else if applied { "Direct provider selected and applied." } else { "Direct provider remains selected. Existing live edits were preserved." }})))
    }).await;
    match result {
        Ok(Ok(None)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Provider not found"})),
        )
            .into_response(),
        Ok(Ok(Some(result))) => Json(result).into_response(),
        Ok(Err(error)) => operation_error(error),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
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
        let (_, status) =
            body(crate::proxy::handlers::proxy_status(State(state.clone())).await).await;
        assert_eq!(status["active_target_provider_id"], "next");
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
        // No direct selection: recover the original owned fields from the backup.
        cc_switch_lib::settings::set_current_provider(&AppType::ClaudeCode, None).unwrap();
        db.set_current_provider("", APP_TYPE).unwrap();
        std::fs::write(&path, r#"{"env":{"ANTHROPIC_BASE_URL":"https://before.example","ANTHROPIC_AUTH_TOKEN":"before","API_TIMEOUT_MS":"99"}}"#).unwrap();
        let (_, started) = body(proxy_start(State(state.clone())).await).await;
        assert_eq!(started["success"], true, "{started}");
        let (_, stopped) = body(proxy_stop(State(state.clone())).await).await;
        assert_eq!(stopped["success"], true, "{stopped}");
        let restored: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(restored["env"]["ANTHROPIC_AUTH_TOKEN"], "before");
        assert_eq!(restored["env"]["API_TIMEOUT_MS"], "99");
        assert_eq!(restored["env"].get("CLAUDE_CODE_AUTO_MODE_SERVER"), None);
    }
    #[tokio::test]
    async fn editing_current_provider_applies_without_losing_keys_on_switch() {
        let _env = TEST_ENV_LOCK.lock().await;
        let home = TestHome::new();
        let db = Arc::new(Database::init().unwrap());
        let provider: Provider = serde_json::from_value(json!({"id":"editor-a", "name":"A", "settingsConfig":{"env":{"ANTHROPIC_AUTH_TOKEN":"old", "ANTHROPIC_BASE_URL":"https://a.example", "ANTHROPIC_MODEL":"old-model"}}, "meta":{"apiFormat":"anthropic"}})).unwrap();
        let mut other = provider.clone();
        other.id = "editor-b".into();
        db.save_provider(APP_TYPE, &provider).unwrap();
        db.save_provider(APP_TYPE, &other).unwrap();
        db.set_current_provider(&provider.id, APP_TYPE).unwrap();
        let state = Arc::new(AppState {
            db: db.clone(),
            token: "test".into(),
            codex_oauth: Arc::new(CodexOAuthManager::new(home.path.clone())),
            copilot_oauth: Arc::new(CopilotAuthManager::new(home.path.clone())),
            proxy_server: Arc::new(tokio::sync::RwLock::new(None)),
            proxy_host: "127.0.0.1".into(),
            proxy_listen_port: 0,
        });
        let (status, _) = body(
            save_provider(
                State(state.clone()),
                Json(ProviderSaveRequest {
                    provider: provider.clone(),
                    base: None,
                    create: true,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        let path = live::get_live_settings_path();
        cc_switch_lib::config::write_json_file(&path, &json!({"env":{"ANTHROPIC_AUTH_TOKEN":"old", "ANTHROPIC_BASE_URL":"https://a.example", "ANTHROPIC_MODEL":"external-model", "API_TIMEOUT_MS":"777"}, "permissions":{"allow":["Read"]}})).unwrap();
        let mut edited = provider.clone();
        edited.settings_config["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("new");
        let (status, result) = body(
            save_provider(
                State(state.clone()),
                Json(ProviderSaveRequest {
                    create: false,
                    provider: edited.clone(),
                    base: Some(provider.clone()),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{result}");
        assert_eq!(result["applied"], true);
        let saved: Value = cc_switch_lib::config::read_json_file(&path).unwrap();
        assert_eq!(saved["env"]["ANTHROPIC_AUTH_TOKEN"], "new");
        assert_eq!(saved["env"]["API_TIMEOUT_MS"], "777");
        assert_eq!(saved["env"]["ANTHROPIC_MODEL"], "external-model");
        assert_eq!(saved["permissions"]["allow"][0], "Read");
        edited.name = "Stale editor change".into();
        assert_eq!(
            body(
                save_provider(
                    State(state.clone()),
                    Json(ProviderSaveRequest {
                        create: false,
                        provider: edited,
                        base: Some(provider)
                    })
                )
                .await
            )
            .await
            .0,
            StatusCode::CONFLICT
        );
        assert_eq!(
            body(switch_provider(State(state.clone()), Path(other.id.clone())).await)
                .await
                .0,
            StatusCode::OK
        );
        assert_eq!(
            db.get_provider("editor-a", APP_TYPE)
                .unwrap()
                .unwrap()
                .settings_config["env"]["ANTHROPIC_AUTH_TOKEN"],
            "new"
        );
        assert_eq!(
            body(switch_provider(State(state.clone()), Path("editor-a".into())).await)
                .await
                .0,
            StatusCode::OK
        );
        assert_eq!(
            cc_switch_lib::config::read_json_file::<Value>(&path).unwrap()["env"]
                ["ANTHROPIC_AUTH_TOKEN"],
            "new"
        );
        let bytes = std::fs::read(&path).unwrap();
        let mut inactive = other.clone();
        inactive.name = "Edited B".into();
        let (_, result) = body(
            update_provider(
                State(state.clone()),
                Path(inactive.id.clone()),
                Json(ProviderSaveRequest {
                    create: false,
                    provider: inactive,
                    base: Some(other),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(result["applied"], false);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let old = db.get_provider("editor-a", APP_TYPE).unwrap().unwrap();
        let mut conflicting = old.clone();
        conflicting.settings_config["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("form-edit");
        let mut live_value: Value = cc_switch_lib::config::read_json_file(&path).unwrap();
        live_value["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("external-key");
        cc_switch_lib::config::write_json_file(&path, &live_value).unwrap();
        assert_eq!(
            body(
                save_provider(
                    State(state.clone()),
                    Json(ProviderSaveRequest {
                        create: false,
                        provider: conflicting,
                        base: Some(old.clone())
                    })
                )
                .await
            )
            .await
            .0,
            StatusCode::CONFLICT
        );
        assert_eq!(
            db.get_provider("editor-a", APP_TYPE)
                .unwrap()
                .unwrap()
                .settings_config,
            old.settings_config
        );
        assert_eq!(
            cc_switch_lib::config::read_json_file::<Value>(&path).unwrap(),
            live_value
        );
        assert!(!cc_switch_lib::provider_operations::recovery_pending());
        // Route takeover must remain intact while editing the direct selection.
        cc_switch_lib::config::write_json_file(&path, &old.settings_config).unwrap();
        db.set_proxy_target_provider_id("editor-b").unwrap();
        let (_, started) = body(proxy_start(State(state.clone())).await).await;
        assert_eq!(started["success"], true, "{started}");
        let takeover = std::fs::read(&path).unwrap();
        let mut routed_edit = old.clone();
        routed_edit.settings_config["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("after-route");
        let (_, result) = body(
            save_provider(
                State(state.clone()),
                Json(ProviderSaveRequest {
                    create: false,
                    provider: routed_edit,
                    base: Some(old),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(result["saved"], true);
        assert_eq!(result["applied"], false);
        assert_eq!(std::fs::read(&path).unwrap(), takeover);
        let (_, stopped) = body(proxy_stop(State(state.clone())).await).await;
        assert_eq!(stopped["success"], true, "{stopped}");
        assert_eq!(
            cc_switch_lib::config::read_json_file::<Value>(&path).unwrap()["env"]
                ["ANTHROPIC_AUTH_TOKEN"],
            "after-route"
        );
    }
}
