//! Provider handlers

use super::super::state::AppState;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use cc_switch_lib::database::Provider;
use serde_json::{json, Value};
use std::sync::Arc;

const APP_TYPE: &str = "claude";
const PROXY_TOKEN_PLACEHOLDER: &str = "PROXY_MANAGED";

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
    log::debug!("[Providers] get_current_provider requested app_type={}", APP_TYPE);
    match state.db.get_current_provider_id(APP_TYPE) {
        Ok(id) => {
            log::debug!("[Providers] get_current_provider success app_type={} current_provider_id={:?}", APP_TYPE, id);
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
    Json(provider): Json<Provider>,
) -> impl IntoResponse {
    log::info!(
        "[Providers] save_provider requested id={} name={}",
        provider.id,
        provider.name
    );
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
    Json(provider): Json<Provider>,
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
    // Get provider
    let provider_result = state.db.get_provider(&id, APP_TYPE);
    let provider = match provider_result {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Provider not found"})),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    // Apply to live config. Codex OAuth providers must point Claude Code at
    // the local proxy; the proxy then injects real OAuth credentials upstream.
    let live_settings = settings_for_live(&provider, state.proxy_listen_port, true);
    let apply_result = cc_switch_lib::live::apply_provider_to_live(&live_settings);
    if let Err(err) = apply_result {
        log::error!("Failed to apply provider to live config: {}", err);
        let err_msg = format!("Failed to apply config: {}", err);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": err_msg})),
        )
            .into_response();
    }

    // Update current provider and local route target in database.
    match state.db.set_current_provider(&id, APP_TYPE) {
        Ok(()) => {}
        Err(e) => {
            log::error!("Failed to switch provider: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    }

    // Also update route target so the local route follows the current provider.
    if let Err(e) = state.db.set_proxy_target_provider_id(&id) {
        log::warn!("Failed to update route target: {}", e);
        // Non-fatal - continue with provider switch
    }
    log::info!(
        "[Providers] switch_provider success id={} proxy_target_updated=true",
        id
    );

    Json(serde_json::json!({ "success": true })).into_response()
}

pub(crate) fn settings_for_live(provider: &Provider, proxy_port: u16, use_proxy: bool) -> Value {
    let mut settings = provider.settings_config.clone();

    if !settings.is_object() {
        settings = json!({});
    }
    let root = settings
        .as_object_mut()
        .expect("settings should be normalized to object");
    let env = root.entry("env".to_string()).or_insert_with(|| json!({}));
    if !env.is_object() {
        *env = json!({});
    }
    let env = env
        .as_object_mut()
        .expect("settings env should be normalized to object");

    if use_proxy {
        if is_codex_oauth_provider(provider) || is_copilot_oauth_provider(provider) {
            // OAuth Provider: 写入代理地址，token 由代理注入
            env.insert(
                "ANTHROPIC_BASE_URL".to_string(),
                json!(format!("http://127.0.0.1:{}", proxy_port)),
            );
            env.insert(
                "ANTHROPIC_AUTH_TOKEN".to_string(),
                json!(PROXY_TOKEN_PLACEHOLDER),
            );
            env.remove("ANTHROPIC_API_KEY");
        } else {
            // Direct Provider: 也写入代理地址，原样转发 API Key
            env.insert(
                "ANTHROPIC_BASE_URL".to_string(),
                json!(format!("http://127.0.0.1:{}", proxy_port)),
            );
            // 保留原有的 API Key，代理会原样转发
            if env
                .get("ANTHROPIC_AUTH_TOKEN")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty())
            {
                env.remove("ANTHROPIC_API_KEY");
            } else if env
                .get("ANTHROPIC_API_KEY")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty())
            {
                env.remove("ANTHROPIC_AUTH_TOKEN");
            }
        }
    }
    settings
}

fn is_codex_oauth_provider(provider: &Provider) -> bool {
    provider.meta.get("providerType").and_then(Value::as_str) == Some("codex_oauth")
}

fn is_copilot_oauth_provider(provider: &Provider) -> bool {
    provider
        .meta
        .get("providerType")
        .and_then(Value::as_str)
        .map(|t| t.contains("copilot"))
        .unwrap_or(false)
        || provider.id.contains("copilot")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn provider_with_env(env: serde_json::Value) -> Provider {
        Provider {
            id: "minimax".to_string(),
            name: "MiniMax".to_string(),
            settings_config: json!({ "env": env }),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            icon: None,
            icon_color: None,
            meta: json!({ "providerType": "minimax" }),
            in_failover_queue: false,
        }
    }

    #[test]
    fn proxy_mode_rewrites_base_url_to_local_proxy() {
        let provider = provider_with_env(json!({
            "ANTHROPIC_BASE_URL": "https://api.minimaxi.com/v1",
            "ANTHROPIC_AUTH_TOKEN": "real-token"
        }));

        let settings = settings_for_live(&provider, 15721, true);
        let env = settings
            .get("env")
            .and_then(|value| value.as_object())
            .unwrap();

        assert_eq!(
            env.get("ANTHROPIC_BASE_URL")
                .and_then(|value| value.as_str()),
            Some("http://127.0.0.1:15721")
        );
        assert_eq!(
            env.get("ANTHROPIC_AUTH_TOKEN")
                .and_then(|value| value.as_str()),
            Some("real-token")
        );
    }

    #[test]
    fn restore_mode_preserves_direct_provider_settings() {
        let provider = provider_with_env(json!({
            "ANTHROPIC_BASE_URL": "https://api.minimaxi.com/v1",
            "ANTHROPIC_AUTH_TOKEN": "real-token"
        }));

        let settings = settings_for_live(&provider, 15721, false);
        let env = settings
            .get("env")
            .and_then(|value| value.as_object())
            .unwrap();

        assert_eq!(
            env.get("ANTHROPIC_BASE_URL")
                .and_then(|value| value.as_str()),
            Some("https://api.minimaxi.com/v1")
        );
        assert_eq!(
            env.get("ANTHROPIC_AUTH_TOKEN")
                .and_then(|value| value.as_str()),
            Some("real-token")
        );
    }
}
