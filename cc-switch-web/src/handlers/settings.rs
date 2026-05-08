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
