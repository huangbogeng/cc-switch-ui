//! Proxy handlers

use super::super::state::AppState;
use super::adapters::create_registry;
use super::types::ModelMapping;
use super::{ProxyConfig, ProxyServer};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use cc_switch_lib::database::{Provider, ProxyType};
use cc_switch_lib::live;
use serde::Deserialize;
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;

const APP_TYPE: &str = cc_switch_lib::DEFAULT_APP_TYPE;
const CODEX_RESPONSES_UPSTREAM: &str = "https://chatgpt.com/backend-api/codex/responses";

#[derive(Deserialize)]
pub struct SetProxyTargetRequest {
    provider_id: String,
}

pub async fn proxy_start(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[ProxyAPI] proxy_start requested");
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
    log::info!(
        "[ProxyAPI] proxy_start target provider_id={} provider_name={}",
        target_provider.id,
        target_provider.name
    );

    // Check if provider is supported by any adapter
    let registry = std::sync::Arc::new(create_registry(
        state.codex_oauth.clone(),
        state.copilot_oauth.clone(),
    ));
    if registry.find_for_provider(&target_provider).is_none() {
        return Json(serde_json::json!({
            "success": false,
            "error": format!("The provider '{}' (type: '{}', format: '{}') cannot be routed. Check that the provider has a valid API format configured.",
                target_provider.name,
                target_provider.meta.get("providerType").and_then(|v| v.as_str()).unwrap_or("not set"),
                target_provider.meta.get("apiFormat").and_then(|v| v.as_str()).unwrap_or("not set"))
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

    // Save backup of original live config before overwriting, so we can
    // restore it when the proxy is stopped or the server crashes.
    if let Ok(raw) = std::fs::read_to_string(cc_switch_lib::live::get_live_settings_path()) {
        if let Err(e) = state.db.save_live_backup(APP_TYPE, &provider_id, &raw) {
            log::warn!("[ProxyAPI] failed to save live backup: {}", e);
        } else {
            log::info!("[ProxyAPI] saved live backup for provider_id={}", provider_id);
        }
    }

    let live_settings = live::settings_for_live(&target_provider, state.proxy_listen_port, true);
    if let Err(e) = cc_switch_lib::live::apply_provider_to_live(&live_settings) {
        log::error!(
            "[ProxyAPI] proxy_start failed to apply proxied live settings provider_id={}: {}",
            target_provider.id,
            e
        );
        return Json(serde_json::json!({
            "success": false,
            "error": format!("Failed to apply proxied provider config: {}", e)
        }))
        .into_response();
    }
    if let Err(msg) = ensure_live_base_url_is_proxy(state.proxy_listen_port) {
        log::error!(
            "[ProxyAPI] proxy_start live settings verification failed: {}",
            msg
        );
        return Json(serde_json::json!({
            "success": false,
            "error": format!("Live settings mismatch after proxy start: {}", msg)
        }))
        .into_response();
    }

    match server
        .start(
            registry,
            account_id,
            db,
            provider_id,
            APP_TYPE,
        )
        .await
    {
        Ok(_actual_addr) => {
            *state.proxy_server.write().await = Some(server);
            log::info!(
                "[ProxyAPI] proxy_start success listen_port={} provider_id={}",
                listen_port,
                target_provider.id
            );
            Json(serde_json::json!({"success": true, "listen_addr": format!("http://0.0.0.0:{}", listen_port), "message": "Proxy started"})).into_response()
        }
        Err(e) => {
            log::error!("[ProxyAPI] proxy_start failed: {}", e);
            Json(serde_json::json!({"success": false, "error": e})).into_response()
        }
    }
}

pub async fn proxy_stop(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[ProxyAPI] proxy_stop requested");
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

    let live_settings = live::settings_for_live(&target_provider, state.proxy_listen_port, false);
    log::info!(
        "[ProxyAPI] proxy_stop restoring live settings for provider_id={}",
        target_provider.id
    );
    if let Err(e) = cc_switch_lib::live::apply_provider_to_live(&live_settings) {
        return Json(serde_json::json!({
            "success": false,
            "error": format!("Failed to restore provider config: {}", e)
        }))
        .into_response();
    }

    // Clean up the live backup now that we've restored the original config
    if let Err(e) = state.db.delete_live_backup(APP_TYPE) {
        log::warn!("[ProxyAPI] failed to delete live backup: {}", e);
    }

    let server = state.proxy_server.write().await.take();
    match server {
        Some(s) => {
            if let Err(e) = s.stop().await {
                log::error!("[ProxyAPI] proxy_stop failed: {}", e);
                Json(serde_json::json!({"success": false, "error": e})).into_response()
            } else {
                log::info!("[ProxyAPI] proxy_stop success");
                Json(serde_json::json!({"success": true, "message": "Proxy stopped"}))
                    .into_response()
            }
        }
        None => {
            log::info!("[ProxyAPI] proxy_stop noop: proxy not running");
            Json(serde_json::json!({"success": true, "message": "Proxy stopped"})).into_response()
        }
    }
}

pub async fn proxy_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let running = state.proxy_server.read().await.is_some();
    let active_target = get_active_target_provider(&state).ok().flatten();
    log::debug!(
        "[ProxyAPI] proxy_status requested running={} active_target_provider_id={:?}",
        running,
        active_target.as_ref().map(|provider| &provider.id)
    );
    Json(serde_json::json!({
        "running": running,
        "listen_addr": if running { Some(format!("http://0.0.0.0:{}", state.proxy_listen_port)) } else { None },
        "upstream_url": CODEX_RESPONSES_UPSTREAM,
        "http_proxy_url": global_http_proxy_url(&state),
        "active_target_provider_id": active_target.as_ref().map(|provider| provider.id.clone()),
        "active_target_provider_name": active_target.as_ref().map(|provider| provider.name.clone()),
        "live_base_url": current_live_base_url(),
    })).into_response()
}

pub async fn proxy_target(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::debug!("[ProxyAPI] proxy_target requested");
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
    log::info!(
        "[ProxyAPI] proxy_set_target requested provider_id={}",
        payload.provider_id
    );
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

    let proxy_running = {
        let guard = state.proxy_server.read().await;
        match guard.as_ref() {
            Some(server) => server.is_running().await,
            None => false,
        }
    };

    if proxy_running {
        let guard = state.proxy_server.read().await;
        if let Some(proxy) = guard.as_ref() {
            match proxy.hot_switch_provider(&state.db, &payload.provider_id).await {
                Ok(()) => {
                    log::info!(
                        "[ProxyAPI] proxy_set_target hot-switch success provider_id={}",
                        payload.provider_id
                    );
                    return Json(serde_json::json!({"success": true})).into_response();
                }
                Err(e) => {
                    log::error!(
                        "[ProxyAPI] proxy_set_target hot-switch failed provider_id={} error={}",
                        payload.provider_id,
                        e
                    );
                    return Json(serde_json::json!({"success": false, "error": e.to_string()}))
                        .into_response();
                }
            }
        }
    }

    match state.db.set_proxy_target_provider_id(&payload.provider_id) {
        Ok(()) => {
            log::info!(
                "[ProxyAPI] proxy_set_target success provider_id={}",
                payload.provider_id
            );
            Json(serde_json::json!({"success": true})).into_response()
        }
        Err(e) => {
            log::error!(
                "[ProxyAPI] proxy_set_target failed provider_id={} error={}",
                payload.provider_id,
                e
            );
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

pub(crate) fn provider_codex_account_id(provider: &Provider) -> Option<String> {
    cc_switch_lib::providers::resolve_managed_account_id(provider)
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

fn ensure_live_base_url_is_proxy(proxy_port: u16) -> Result<(), String> {
    let expected = format!("http://127.0.0.1:{proxy_port}");
    let actual = current_live_base_url().unwrap_or_default();
    if actual == expected {
        return Ok(());
    }
    Err(format!(
        "expected base_url={}, got base_url={}",
        expected, actual
    ))
}

fn current_live_base_url() -> Option<String> {
    let path = cc_switch_lib::live::get_live_settings_path();
    let raw = std::fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&raw).ok()?;
    // Provider settings are stored under "env" in the live config
    value
        .get("env")
        .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
        .and_then(Value::as_str)
        .map(str::to_string)
}
