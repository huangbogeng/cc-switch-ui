//! Settings handlers for proxy configuration

use axum::{extract::State, http::StatusCode, Json};
use axum::response::IntoResponse;
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
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfigRequest {
    enabled: bool,
    proxy_type: String,
    host: String,
    port: u16,
}

pub async fn get_proxy_config(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.get_proxy_config() {
        Ok(Some(config)) => {
            Json(ProxyConfigResponse::from(&config)).into_response()
        }
        Ok(None) => {
            // No proxy configured, return default disabled state
            Json(serde_json::json!({
                "enabled": false,
                "proxyType": "http",
                "host": "",
                "port": 10809
            })).into_response()
        }
        Err(e) => {
            log::error!("Failed to get proxy config: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

pub async fn set_proxy_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ProxyConfigRequest>,
) -> impl IntoResponse {
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
    };

    if let Err(e) = state.db.set_proxy_config(&config) {
        log::error!("Failed to save proxy config: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }

    // Update OAuth managers with new proxy config
    if config.enabled {
        state.codex_oauth.set_proxy_config(&config).await;
        state.copilot_oauth.set_proxy_config(&config).await;
    }

    log::info!("[Settings] Proxy config updated: {}://{}:{}",
        payload.proxy_type, host, payload.port);

    Json(serde_json::json!({"success": true})).into_response()
}

pub async fn delete_proxy_config(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if let Err(e) = state.db.delete_proxy_config() {
        log::error!("Failed to delete proxy config: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }

    // Reset OAuth managers to use default HTTP client (no proxy)
    let default_config = ProxyConfig::default();
    state.codex_oauth.set_proxy_config(&default_config).await;
    state.copilot_oauth.set_proxy_config(&default_config).await;

    log::info!("[Settings] Proxy config deleted");

    Json(serde_json::json!({"success": true})).into_response()
}
