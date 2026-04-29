//! Proxy handlers

use std::net::SocketAddr;
use std::sync::Arc;
use axum::{extract::State, Json};
use axum::response::IntoResponse;
use serde::Deserialize;
use super::{ProxyConfig, ProxyServer};
use super::super::state::AppState;

const APP_TYPE: &str = "claude";

#[derive(Deserialize)]
pub struct SetProxyTargetRequest {
    provider_id: String,
}

pub async fn proxy_start(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if state.proxy_server.read().await.is_some() {
        return Json(serde_json::json!({"success": false, "error": "Proxy already running"})).into_response();
    }

    let target_provider = match get_active_target_provider(&state) {
        Ok(Some(provider)) => provider,
        Ok(None) => {
            return Json(serde_json::json!({
                "success": false,
                "error": "No proxy target selected. Choose a Codex OAuth provider first."
            })).into_response();
        }
        Err(e) => {
            return Json(serde_json::json!({"success": false, "error": e.to_string()})).into_response();
        }
    };

    if !is_codex_oauth_provider(&target_provider) {
        return Json(serde_json::json!({
            "success": false,
            "error": "The active proxy target is not a Codex OAuth provider."
        })).into_response();
    }

    let status = state.codex_oauth.get_status().await;
    if !status.authenticated {
        return Json(serde_json::json!({"success": false, "error": "Not authenticated. Please complete OAuth first."})).into_response();
    }

    let proxy_addr = SocketAddr::from(([0, 0, 0, 0], state.proxy_listen_port));
    let upstream_url = provider_base_url(&target_provider)
        .unwrap_or_else(|| "https://chatgpt.com/backend-api/codex".to_string());
    let config = ProxyConfig {
        listen_addr: proxy_addr,
        upstream_url,
        http_proxy_url: provider_codex_http_proxy(&target_provider),
    };
    let server = ProxyServer::new(config);
    let listen_port = state.proxy_listen_port;

    let account_id = provider_codex_account_id(&target_provider);
    match server.start(state.codex_oauth.clone(), account_id).await {
        Ok(_actual_addr) => {
            *state.proxy_server.write().await = Some(server);
            Json(serde_json::json!({"success": true, "listen_addr": format!("http://0.0.0.0:{}", listen_port), "message": "Proxy started"})).into_response()
        }
        Err(e) => Json(serde_json::json!({"success": false, "error": e})).into_response()
    }
}

pub async fn proxy_stop(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let server = state.proxy_server.write().await.take();
    match server {
        Some(s) => {
            if let Err(e) = s.stop().await {
                Json(serde_json::json!({"success": false, "error": e})).into_response()
            } else {
                Json(serde_json::json!({"success": true, "message": "Proxy stopped"})).into_response()
            }
        }
        None => Json(serde_json::json!({"success": false, "error": "Proxy not running"})).into_response()
    }
}

pub async fn proxy_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let running = state.proxy_server.read().await.is_some();
    let active_target = get_active_target_provider(&state).ok().flatten();
    Json(serde_json::json!({
        "running": running,
        "listen_addr": if running { Some(format!("http://0.0.0.0:{}", state.proxy_listen_port)) } else { None },
        "upstream_url": active_target.as_ref()
            .and_then(provider_base_url)
            .unwrap_or_else(|| "https://chatgpt.com/backend-api/codex".to_string()),
        "http_proxy_url": active_target.as_ref().and_then(provider_codex_http_proxy),
        "active_target_provider_id": active_target.as_ref().map(|provider| provider.id.clone()),
        "active_target_provider_name": active_target.as_ref().map(|provider| provider.name.clone()),
    })).into_response()
}

pub async fn proxy_target(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match get_active_target_provider(&state) {
        Ok(provider) => Json(serde_json::json!({
            "provider_id": provider.as_ref().map(|provider| provider.id.clone()),
            "provider": provider,
        })).into_response(),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})).into_response(),
    }
}

pub async fn proxy_set_target(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetProxyTargetRequest>,
) -> impl IntoResponse {
    let provider = match state.db.get_provider(&payload.provider_id, APP_TYPE) {
        Ok(Some(provider)) => provider,
        Ok(None) => return Json(serde_json::json!({"success": false, "error": "Provider not found"})).into_response(),
        Err(e) => return Json(serde_json::json!({"success": false, "error": e.to_string()})).into_response(),
    };

    if !is_codex_oauth_provider(&provider) {
        return Json(serde_json::json!({
            "success": false,
            "error": "Only Codex OAuth providers can be used as the current proxy target."
        })).into_response();
    }

    match state.db.set_proxy_target_provider_id(&payload.provider_id) {
        Ok(()) => Json(serde_json::json!({"success": true})).into_response(),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})).into_response(),
    }
}

fn get_active_target_provider(
    state: &AppState,
) -> Result<Option<cc_switch_lib::database::Provider>, cc_switch_lib::error::AppError> {
    let target_id = state
        .db
        .get_proxy_target_provider_id()?
        .or(state.db.get_current_provider_id(APP_TYPE)?);
    match target_id {
        Some(id) => state.db.get_provider(&id, APP_TYPE),
        None => Ok(None),
    }
}

fn is_codex_oauth_provider(provider: &cc_switch_lib::database::Provider) -> bool {
    provider
        .meta
        .get("providerType")
        .and_then(|value| value.as_str())
        == Some("codex_oauth")
}

fn provider_base_url(provider: &cc_switch_lib::database::Provider) -> Option<String> {
    provider
        .settings_config
        .get("env")
        .and_then(|value| value.get("ANTHROPIC_BASE_URL"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn provider_codex_account_id(provider: &cc_switch_lib::database::Provider) -> Option<String> {
    provider
        .meta
        .get("authBinding")
        .and_then(|value| value.get("accountId"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn provider_codex_http_proxy(provider: &cc_switch_lib::database::Provider) -> Option<String> {
    provider
        .meta
        .get("codexHttpProxy")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}
