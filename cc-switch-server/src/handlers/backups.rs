use crate::state::{AppState, PROVIDER_MUTATIONS};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use cc_switch_lib::{database::backups, error::AppError};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupRequest {
    id: String,
}
fn result(value: Result<serde_json::Value, AppError>) -> Response {
    match value {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            match &error {
                AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
                AppError::ProviderInUse(_) | AppError::Conflict { .. } => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Json(serde_json::json!({"error":error.to_string()})),
        )
            .into_response(),
    }
}
fn json(value: impl serde::Serialize) -> Result<serde_json::Value, AppError> {
    serde_json::to_value(value).map_err(|source| AppError::JsonSerialize { source })
}
pub async fn list_files() -> Response {
    super::file_mutation(|| {
        result(
            cc_switch_lib::config::backups::list()
                .and_then(|items| json(serde_json::json!({"backups":items}))),
        )
    })
    .await
}

pub async fn list() -> Response {
    super::file_mutation(|| result(backups::status().and_then(json))).await
}
pub async fn create(State(state): State<Arc<AppState>>) -> Response {
    let _mutation = PROVIDER_MUTATIONS.lock().await;
    if state.proxy_server.read().await.is_some() {
        return result(Err(AppError::ProviderInUse(
            "Stop the local route before creating a database backup".into(),
        )));
    }
    super::file_mutation(move || {
        result((|| {
            ensure_idle(&state)?;
            let id = backups::create(&state.db)?;
            Ok(serde_json::json!({"success":true,"id":id}))
        })())
    })
    .await
}
fn ensure_idle(state: &AppState) -> Result<(), AppError> {
    if cc_switch_lib::provider_operations::recovery_pending()
        || state
            .db
            .get_live_backup(cc_switch_lib::DEFAULT_APP_TYPE)?
            .is_some()
    {
        return Err(AppError::ProviderInUse(
            "Recover unfinished provider or route operations before using database backups".into(),
        ));
    }
    Ok(())
}
pub async fn schedule(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BackupRequest>,
) -> Response {
    let _mutation = PROVIDER_MUTATIONS.lock().await;
    if state.proxy_server.read().await.is_some() {
        return result(Err(AppError::ProviderInUse(
            "Stop the local route before restoring a database backup".into(),
        )));
    }
    super::file_mutation(move || {
        result((|| {
            ensure_idle(&state)?;
            backups::schedule(&request.id)?;
            Ok(serde_json::json!({"success":true,"restartRequired":true}))
        })())
    })
    .await
}
pub async fn delete(Json(request): Json<BackupRequest>) -> Response {
    super::file_mutation(move || {
        result(backups::delete(&request.id).map(|_| serde_json::json!({"success":true})))
    })
    .await
}
pub async fn cancel() -> Response {
    super::file_mutation(|| result(backups::cancel().map(|_| serde_json::json!({"success":true}))))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        proxy::handlers::proxy_start,
        state::{TestHome, TEST_ENV_LOCK},
    };
    use cc_switch_lib::{
        database::{Database, Provider},
        oauth::{codex::CodexOAuthManager, copilot::CopilotAuthManager},
    };
    use serde_json::{json, Value};
    async fn body(response: impl IntoResponse) -> (StatusCode, Value) {
        let response = response.into_response();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1000000)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    #[tokio::test]
    async fn file_inventory_is_fixed_metadata_only_and_isolates_invalid_snapshots() {
        let _env = TEST_ENV_LOCK.lock().await;
        let _home = TestHome::new();
        let path = cc_switch_lib::config::get_claude_settings_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"inventory-secret"}}"#,
        )
        .unwrap();
        cc_switch_lib::config::update_json_file(&path, |value| {
            value["changed"] = json!(true);
            Ok(())
        })
        .unwrap();
        let (status, value) = body(list_files().await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(value["backups"].as_array().unwrap().len(), 3);
        assert_eq!(value["backups"][0]["status"], "available");
        assert!(!value.to_string().contains("inventory-secret"));
        let original = value["backups"][0]["originalPath"].as_str().unwrap();
        std::fs::remove_file(original).unwrap();
        let (_, value) = body(list_files().await).await;
        assert_eq!(value["backups"][0]["status"], "invalid");
        assert_eq!(value["backups"][1]["status"], "missing");
        assert_eq!(value["backups"][2]["status"], "missing");
    }

    #[tokio::test]
    async fn backup_api_rejects_route_recovery_and_pending_restore_blocks_route_start() {
        let _env = TEST_ENV_LOCK.lock().await;
        let home = TestHome::new();
        let db = Arc::new(Database::init().unwrap());
        let provider:Provider=serde_json::from_value(json!({"id":"backup-api", "name":"Test", "settingsConfig":{"env":{"ANTHROPIC_AUTH_TOKEN":"key"}},"meta":{"apiFormat":"anthropic"}})).unwrap();
        db.save_provider(cc_switch_lib::DEFAULT_APP_TYPE, &provider)
            .unwrap();
        let state = Arc::new(AppState {
            db: db.clone(),
            token: "test".into(),
            codex_oauth: Arc::new(CodexOAuthManager::new(home.path.clone())),
            copilot_oauth: Arc::new(CopilotAuthManager::new(home.path.clone())),
            proxy_server: Arc::new(tokio::sync::RwLock::new(None)),
            proxy_host: "127.0.0.1".into(),
            proxy_listen_port: 0,
        });
        db.save_live_backup(cc_switch_lib::DEFAULT_APP_TYPE, &provider.id, "{}")
            .unwrap();
        assert_eq!(
            body(create(State(state.clone())).await).await.0,
            StatusCode::CONFLICT
        );
        db.delete_live_backup(cc_switch_lib::DEFAULT_APP_TYPE)
            .unwrap();
        let (status, result) = body(create(State(state.clone())).await).await;
        assert_eq!(status, StatusCode::OK);
        let id = result["id"].as_str().unwrap().to_string();
        assert_eq!(
            body(
                schedule(
                    State(state.clone()),
                    Json(BackupRequest {
                        id: "../../cc-switch.db".into()
                    })
                )
                .await
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
        let (_, result) =
            body(schedule(State(state.clone()), Json(BackupRequest { id: id.clone() })).await)
                .await;
        assert_eq!(result["restartRequired"], true);
        let (_, result) = body(proxy_start(State(state.clone())).await).await;
        assert_eq!(result["success"], false);
        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("planned database restore"));
        assert!(state.proxy_server.read().await.is_none());
        assert_eq!(
            body(delete(Json(BackupRequest { id })).await).await.0,
            StatusCode::CONFLICT
        );
        let (_, result) = body(list().await).await;
        assert!(result["pending"].get("digest").is_none());
        assert!(result["pending"].get("databasePath").is_none());
        assert_eq!(body(cancel().await).await.0, StatusCode::OK);
        assert!(!backups::restore_pending().unwrap());
    }
}
