//! Settings handlers for proxy configuration

use axum::response::IntoResponse;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use cc_switch_lib::database::{ProxyConfig, ProxyType};

use super::super::state::AppState;

#[derive(Serialize)]
pub struct ProxyConfigResponse {
    enabled: bool,
    proxy_type: String,
    host: String,
    port: u16,
    auto_failover_enabled: bool,
}

impl From<&ProxyConfig> for ProxyConfigResponse {
    fn from(config: &ProxyConfig) -> Self {
        Self {
            enabled: config.enabled,
            proxy_type: match config.proxy_type {
                ProxyType::Http => "http".to_string(),
                ProxyType::Socks5 => "socks5".to_string(),
            },
            host: config.host.clone(),
            port: config.port,
            auto_failover_enabled: config.auto_failover_enabled,
        }
    }
}

#[derive(Deserialize)]
pub struct ProxyConfigRequest {
    enabled: bool,
    #[serde(rename = "proxy_type", alias = "proxyType")]
    proxy_type: String,
    host: String,
    port: u16,
    #[serde(default, alias = "autoFailoverEnabled")]
    auto_failover_enabled: bool,
}

pub async fn get_proxy_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[Settings] get_proxy_config requested");
    match state.db.get_proxy_config() {
        Ok(Some(config)) => Json(ProxyConfigResponse::from(&config)).into_response(),
        Ok(None) => Json(serde_json::json!({
            "enabled": false,
            "proxy_type": "http",
            "host": "",
            "port": 10809,
            "auto_failover_enabled": false
        }))
        .into_response(),
        Err(e) => {
            log::error!("Failed to get proxy config: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn set_proxy_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ProxyConfigRequest>,
) -> impl IntoResponse {
    log::info!(
        "[Settings] set_proxy_config requested enabled={} proxy_type={} host={} port={} auto_failover_enabled={}",
        payload.enabled,
        payload.proxy_type,
        payload.host,
        payload.port,
        payload.auto_failover_enabled
    );
    let proxy_type = match payload.proxy_type.to_lowercase().as_str() {
        "socks5" | "socks" => ProxyType::Socks5,
        _ => ProxyType::Http,
    };

    let host = payload.host.clone();
    let config = ProxyConfig {
        enabled: payload.enabled,
        proxy_type,
        host: payload.host,
        port: payload.port,
        auto_failover_enabled: payload.auto_failover_enabled,
    };

    if let Err(e) = state.db.set_proxy_config(&config) {
        log::error!("Failed to save proxy config: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }

    state.codex_oauth.set_proxy_config(&config).await;
    state.copilot_oauth.set_proxy_config(&config).await;

    log::info!(
        "[Settings] Proxy config updated: {}://{}:{}",
        payload.proxy_type,
        host,
        payload.port
    );

    Json(serde_json::json!({"success": true})).into_response()
}

pub async fn delete_proxy_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[Settings] delete_proxy_config requested");
    if let Err(e) = state.db.delete_proxy_config() {
        log::error!("Failed to delete proxy config: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }

    let default_config = ProxyConfig::default();
    state.codex_oauth.set_proxy_config(&default_config).await;
    state.copilot_oauth.set_proxy_config(&default_config).await;

    log::info!("[Settings] Proxy config deleted");

    Json(serde_json::json!({"success": true})).into_response()
}

/// Test proxy connectivity by attempting to reach auth.openai.com through the proxy
#[allow(dead_code)]
pub async fn test_proxy_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let proxy_config = match state.db.get_proxy_config() {
        Ok(Some(config)) => config,
        Ok(None) => {
            return Json(serde_json::json!({
                "success": false,
                "error": "No proxy configured"
            }))
            .into_response();
        }
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to get proxy config: {}", e)
            }))
            .into_response();
        }
    };

    if !proxy_config.enabled {
        return Json(serde_json::json!({
            "success": false,
            "error": "Proxy is disabled"
        }))
        .into_response();
    }

    let client = match cc_switch_lib::oauth::new_http_client_with_proxy(&proxy_config) {
        Ok(c) => c,
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to create HTTP client: {}", e)
            }))
            .into_response();
        }
    };

    let test_url = "https://auth.openai.com/";

    match client.get(test_url).send().await {
        Ok(response) => {
            let status = response.status();
            if status.is_success() || status.as_u16() == 401 {
                Json(serde_json::json!({
                    "success": true,
                    "message": format!("Proxy works! Server responded with status {}", status)
                })).into_response()
            } else {
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Server returned status {}", status)
                })).into_response()
            }
        }
        Err(e) => {
            Json(serde_json::json!({
                "success": false,
                "error": format!("Connection failed: {}. Check if your proxy is running and accessible.", e)
            })).into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct ProxyPortRequest {
    port: u16,
}

pub async fn get_proxy_port(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[Settings] get_proxy_port requested");
    match state.db.get_proxy_port() {
        Ok(port) => Json(serde_json::json!({ "port": port })).into_response(),
        Err(e) => {
            log::error!("Failed to get proxy port: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn set_proxy_port(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ProxyPortRequest>,
) -> impl IntoResponse {
    log::info!("[Settings] set_proxy_port requested port={}", payload.port);
    if let Err(e) = state.db.set_proxy_port(payload.port) {
        log::error!("Failed to save proxy port: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }

    log::info!("[Settings] Proxy port updated: {}", payload.port);

    Json(serde_json::json!({"success": true})).into_response()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestoreUserSettingsRequest {
    revision: String,
}

fn user_settings_error(error: cc_switch_lib::error::AppError) -> axum::response::Response {
    use cc_switch_lib::error::AppError;
    let status = match &error {
        AppError::Conflict { .. } | AppError::SettingsConflict { .. } => StatusCode::CONFLICT,
        AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let mut body = serde_json::json!({"error": error.to_string()});
    if let AppError::SettingsConflict { keys, revision } = &error {
        body["conflictKeys"] = serde_json::json!(keys);
        body["conflictRevision"] = serde_json::json!(revision);
    }
    (status, Json(body)).into_response()
}

pub async fn get_claude_user_settings() -> axum::response::Response {
    match tokio::task::spawn_blocking(|| {
        cc_switch_lib::live::user_settings::read(&cc_switch_lib::config::get_claude_settings_path())
    })
    .await
    {
        Ok(Ok(settings)) => Json(settings).into_response(),
        Ok(Err(error)) => user_settings_error(error),
        Err(error) => {
            user_settings_error(cc_switch_lib::error::AppError::Message(error.to_string()))
        }
    }
}

pub async fn save_claude_user_settings(
    Json(request): Json<cc_switch_lib::live::user_settings::SaveRequest>,
) -> axum::response::Response {
    match tokio::task::spawn_blocking(move || {
        cc_switch_lib::live::user_settings::save(
            &cc_switch_lib::config::get_claude_settings_path(),
            &request,
        )
    })
    .await
    {
        Ok(Ok(())) => Json(serde_json::json!({"success": true})).into_response(),
        Ok(Err(error)) => user_settings_error(error),
        Err(error) => {
            user_settings_error(cc_switch_lib::error::AppError::Message(error.to_string()))
        }
    }
}

pub async fn merge_claude_user_settings(
    Json(request): Json<cc_switch_lib::live::user_settings::MergeRequest>,
) -> axum::response::Response {
    super::file_mutation(move || {
        use axum::response::IntoResponse;
        match cc_switch_lib::live::user_settings::merge(
            &cc_switch_lib::config::get_claude_settings_path(),
            &request,
        ) {
            Ok(()) => Json(serde_json::json!({"success": true})).into_response(),
            Err(error) => user_settings_error(error),
        }
    })
    .await
}

pub async fn restore_claude_user_settings(
    Json(request): Json<RestoreUserSettingsRequest>,
) -> axum::response::Response {
    match tokio::task::spawn_blocking(move || {
        cc_switch_lib::live::user_settings::restore(
            &cc_switch_lib::config::get_claude_settings_path(),
            &request.revision,
        )
    })
    .await
    {
        Ok(Ok(())) => Json(serde_json::json!({"success": true})).into_response(),
        Ok(Err(error)) => user_settings_error(error),
        Err(error) => {
            user_settings_error(cc_switch_lib::error::AppError::Message(error.to_string()))
        }
    }
}

#[cfg(test)]
mod user_settings_tests {
    use super::*;
    use cc_switch_lib::live::user_settings::{SaveRequest, UserValues};
    use serde_json::{json, Value};

    async fn body(response: axum::response::Response) -> (StatusCode, Value) {
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn field_conflict_response_contains_only_keys_and_revision() {
        let (code, json) = body(user_settings_error(
            cc_switch_lib::AppError::SettingsConflict {
                keys: vec!["API_TIMEOUT_MS".into()],
                revision: "observed".into(),
            },
        ))
        .await;
        assert_eq!(code, StatusCode::CONFLICT);
        assert_eq!(json["conflictKeys"], serde_json::json!(["API_TIMEOUT_MS"]));
        assert_eq!(json["conflictRevision"], "observed");
        assert!(json.get("values").is_none());
    }

    #[tokio::test]
    async fn user_settings_api_preserves_credentials_and_returns_conflicts() {
        let _env = crate::state::TEST_ENV_LOCK.lock().await;
        let _home = crate::state::TestHome::new();
        let path = cc_switch_lib::config::get_claude_settings_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"test-secret","API_TIMEOUT_MS":"10"}}"#,
        )
        .unwrap();
        let (status, loaded) = body(get_claude_user_settings().await).await;
        assert_eq!(status, StatusCode::OK);
        assert!(!loaded.to_string().contains("test-secret"));
        let revision = loaded["revision"].as_str().unwrap().to_string();
        let (status, _) = body(
            save_claude_user_settings(Json(SaveRequest {
                revision: revision.clone(),
                values: UserValues {
                    timeout_ms: Some("0".into()),
                    nonessential_traffic: None,
                },
            }))
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let (status, _) = body(
            save_claude_user_settings(Json(SaveRequest {
                revision: revision.clone(),
                values: UserValues {
                    timeout_ms: Some("20".into()),
                    nonessential_traffic: Some("1".into()),
                },
            }))
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) =
            body(restore_claude_user_settings(Json(RestoreUserSettingsRequest { revision })).await)
                .await;
        assert_eq!(status, StatusCode::CONFLICT);
        let (_, loaded) = body(get_claude_user_settings().await).await;
        assert_eq!(loaded["initialValues"]["timeoutMs"], "10");
        let (status, _) = body(
            restore_claude_user_settings(Json(RestoreUserSettingsRequest {
                revision: loaded["revision"].as_str().unwrap().into(),
            }))
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let current: Value = cc_switch_lib::config::read_json_file(&path).unwrap();
        assert_eq!(current["env"]["API_TIMEOUT_MS"], "10");
        assert_eq!(current["env"]["ANTHROPIC_AUTH_TOKEN"], "test-secret");
        assert_eq!(
            current["env"].get("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"),
            None
        );
        assert_eq!(
            current["env"],
            json!({"ANTHROPIC_AUTH_TOKEN":"test-secret", "API_TIMEOUT_MS":"10"})
        );
    }
}
