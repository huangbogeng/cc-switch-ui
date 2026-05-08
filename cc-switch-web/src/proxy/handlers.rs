//! Proxy handlers

use super::super::state::AppState;
use super::adapters::create_registry;
use super::types::ModelMapping;
use super::{ProxyConfig, ProxyServer};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use cc_switch_lib::database::{Provider, ProxyType};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;

const APP_TYPE: &str = "claude";
const CODEX_RESPONSES_UPSTREAM: &str = "https://chatgpt.com/backend-api/codex/responses";

#[derive(Deserialize)]
pub struct SetProxyTargetRequest {
    provider_id: String,
}

pub async fn proxy_start(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if state.proxy_server.read().await.is_some() {
        return Json(serde_json::json!({"success": false, "error": "Proxy already running"}))
            .into_response();
    }

    let target_provider = match get_active_target_provider(&state) {
        Ok(Some(provider)) => provider,
        Ok(None) => {
            return Json(serde_json::json!({
                "success": false,
                "error": "No route target selected. Choose a provider route first."
            }))
            .into_response();
        }
        Err(e) => {
            return Json(serde_json::json!({"success": false, "error": e.to_string()}))
                .into_response();
        }
    };

    // Check if provider is supported by any adapter
    let registry = create_registry(state.codex_oauth.clone(), state.copilot_oauth.clone());
    if registry.find_for_provider(&target_provider).is_none() {
        return Json(serde_json::json!({
            "success": false,
            "error": format!("The provider type '{}' is not supported by the local route.",
                target_provider.meta.get("providerType").and_then(|v| v.as_str()).unwrap_or("unknown"))
        }))
        .into_response();
    }

    let proxy_addr = SocketAddr::from(([0, 0, 0, 0], state.proxy_listen_port));
    let config = ProxyConfig {
        listen_addr: proxy_addr,
        upstream_url: CODEX_RESPONSES_UPSTREAM.to_string(),
        http_proxy_url: global_http_proxy_url(&state),
        prompt_cache_key: provider_prompt_cache_key(&target_provider),
        prompt_cache_key_fallback: target_provider.id.clone(),
        codex_fast_mode: provider_codex_fast_mode(&target_provider),
        model_mapping: provider_model_mapping(&target_provider),
    };
    let server = ProxyServer::new(config);
    let listen_port = state.proxy_listen_port;

    let account_id = provider_codex_account_id(&target_provider);
    let provider_id = target_provider.id.clone();
    let db = state.db.clone();
    match server
        .start(
            state.codex_oauth.clone(),
            state.copilot_oauth.clone(),
            account_id,
            db,
            provider_id,
            APP_TYPE,
        )
        .await
    {
        Ok(_actual_addr) => {
            *state.proxy_server.write().await = Some(server);
            Json(serde_json::json!({"success": true, "listen_addr": format!("http://0.0.0.0:{}", listen_port), "message": "Proxy started"})).into_response()
        }
        Err(e) => Json(serde_json::json!({"success": false, "error": e})).into_response(),
    }
}

pub async fn proxy_stop(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let server = state.proxy_server.write().await.take();
    match server {
        Some(s) => {
            if let Err(e) = s.stop().await {
                Json(serde_json::json!({"success": false, "error": e})).into_response()
            } else {
                Json(serde_json::json!({"success": true, "message": "Proxy stopped"}))
                    .into_response()
            }
        }
        None => Json(serde_json::json!({"success": false, "error": "Proxy not running"}))
            .into_response(),
    }
}

pub async fn proxy_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let running = state.proxy_server.read().await.is_some();
    let active_target = get_active_target_provider(&state).ok().flatten();
    Json(serde_json::json!({
        "running": running,
        "listen_addr": if running { Some(format!("http://0.0.0.0:{}", state.proxy_listen_port)) } else { None },
        "upstream_url": CODEX_RESPONSES_UPSTREAM,
        "http_proxy_url": global_http_proxy_url(&state),
        "active_target_provider_id": active_target.as_ref().map(|provider| provider.id.clone()),
        "active_target_provider_name": active_target.as_ref().map(|provider| provider.name.clone()),
    })).into_response()
}

pub async fn proxy_target(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match get_active_target_provider(&state) {
        Ok(provider) => Json(serde_json::json!({
            "provider_id": provider.as_ref().map(|provider| provider.id.clone()),
            "provider": provider,
        }))
        .into_response(),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})).into_response(),
    }
}

pub async fn proxy_set_target(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetProxyTargetRequest>,
) -> impl IntoResponse {
    let provider = match state.db.get_provider(&payload.provider_id, APP_TYPE) {
        Ok(Some(provider)) => provider,
        Ok(None) => {
            return Json(serde_json::json!({"success": false, "error": "Provider not found"}))
                .into_response()
        }
        Err(e) => {
            return Json(serde_json::json!({"success": false, "error": e.to_string()}))
                .into_response()
        }
    };

    // Check if provider is supported by any adapter
    let registry = create_registry(state.codex_oauth.clone(), state.copilot_oauth.clone());
    if registry.find_for_provider(&provider).is_none() {
        return Json(serde_json::json!({
            "success": false,
            "error": format!("The provider type '{}' is not supported by the local route.",
                provider.meta.get("providerType").and_then(|v| v.as_str()).unwrap_or("unknown"))
        }))
        .into_response();
    }

    match state.db.set_proxy_target_provider_id(&payload.provider_id) {
        Ok(()) => Json(serde_json::json!({"success": true})).into_response(),
        Err(e) => {
            Json(serde_json::json!({"success": false, "error": e.to_string()})).into_response()
        }
    }
}

fn get_active_target_provider(
    state: &AppState,
) -> Result<Option<Provider>, cc_switch_lib::error::AppError> {
    let target_id = state
        .db
        .get_proxy_target_provider_id()?
        .or(state.db.get_current_provider_id(APP_TYPE)?);
    match target_id {
        Some(id) => state.db.get_provider(&id, APP_TYPE),
        None => Ok(None),
    }
}

fn provider_codex_account_id(provider: &Provider) -> Option<String> {
    provider
        .meta
        .get("authBinding")
        .and_then(|value| value.get("accountId"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn provider_prompt_cache_key(provider: &Provider) -> Option<String> {
    provider
        .meta
        .get("promptCacheKey")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn provider_codex_fast_mode(provider: &Provider) -> bool {
    provider
        .meta
        .get("codexFastMode")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn provider_model_mapping(provider: &Provider) -> ModelMapping {
    let env = provider
        .settings_config
        .get("env")
        .and_then(|value| value.as_object());
    let get = |key: &str| {
        env.and_then(|env| env.get(key))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    };

    ModelMapping {
        default_model: get("ANTHROPIC_MODEL"),
        haiku_model: get("ANTHROPIC_DEFAULT_HAIKU_MODEL"),
        sonnet_model: get("ANTHROPIC_DEFAULT_SONNET_MODEL"),
        opus_model: get("ANTHROPIC_DEFAULT_OPUS_MODEL"),
    }
}

fn global_http_proxy_url(state: &AppState) -> Option<String> {
    let config = state.db.get_proxy_config().ok().flatten()?;
    if !config.enabled || config.host.trim().is_empty() {
        return None;
    }

    let scheme = match config.proxy_type {
        ProxyType::Http => "http",
        ProxyType::Socks5 => "socks5",
    };
    Some(format!(
        "{}://{}:{}",
        scheme,
        config.host.trim(),
        config.port
    ))
}
