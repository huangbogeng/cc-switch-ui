//! Provider handlers

use super::super::state::AppState;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use cc_switch_lib::database::Provider;
use cc_switch_lib::providers::AppType;
use serde_json::{json, Value};
use std::sync::Arc;

const APP_TYPE: &str = "claude_code";
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

    let should_hot_switch = (backup.is_some() || live_taken_over) && proxy_running;
    let mut warnings: Vec<String> = Vec::new();

    if should_hot_switch {
        // Path A: Hot-switch (proxy takeover mode)
        log::info!(
            "[Providers] switch_provider id={} path=hot_switch",
            id
        );

        // Block switching to official providers
        let provider_type = provider
            .meta
            .get("providerType")
            .and_then(Value::as_str);
        if provider_type == Some("official") || provider_type == Some("Official") {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": "Cannot switch to official provider while proxy takeover is active"
                })),
            )
                .into_response();
        }

        // Just update the proxy target — live config already points at proxy
        let guard = state.proxy_server.read().await;
        if let Some(proxy) = guard.as_ref() {
            if let Err(e) = proxy.hot_switch_provider(&state.db, &id).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response();
            }
        }
    } else {
        // Path B: Normal switch (with backfill)
        log::info!(
            "[Providers] switch_provider id={} path=normal",
            id
        );

        // (a) Backfill: save current live config to old provider's DB record
        let app_type_enum: AppType = APP_TYPE.parse().unwrap_or(AppType::ClaudeCode);
        let current_id = cc_switch_lib::settings::get_current_provider(&app_type_enum);
        if let Some(ref old_id) = current_id {
            if old_id != &id {
                match cc_switch_lib::live::backfill_current_live_config() {
                    Ok(backfill_settings) => {
                        match state.db.get_provider(old_id, app_type) {
                            Ok(Some(mut old_provider)) => {
                                old_provider.settings_config = backfill_settings;
                                if let Err(e) =
                                    state.db.save_provider(app_type, &old_provider)
                                {
                                    warnings.push(format!(
                                        "Backfill warning: failed to save old provider config: {}",
                                        e
                                    ));
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
                    Err(e) => {
                        warnings.push(format!(
                            "Backfill warning: failed to read live config: {}",
                            e
                        ));
                    }
                }
            }
        }

        // (b) Set device-level current provider
        if let Err(e) =
            cc_switch_lib::settings::set_current_provider(&app_type_enum, Some(&id))
        {
            warnings.push(format!("Failed to save device setting: {}", e));
        }

        // (c) Set DB is_current
        if let Err(e) = state.db.set_current_provider(&id, app_type) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to set current provider: {}", e)
                })),
            )
                .into_response();
        }

        // (d) Write live config (with sanitize, optional proxy routing)
        let use_proxy = proxy_running;
        let live_settings = settings_for_live(&provider, state.proxy_listen_port, use_proxy);
        if let Err(e) = cc_switch_lib::live::apply_provider_to_live(&live_settings) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to apply config: {}", e)
                })),
            )
                .into_response();
        }

        // (e) Sync MCP servers to ~/.claude.json
        if let Err(e) = cc_switch_lib::mcp::sync_enabled_to_claude(&state.db, app_type) {
            warnings.push(format!("MCP sync warning: {}", e));
        }

        // (f) Sync skills to ~/.claude/skills/
        if let Err(e) = cc_switch_lib::skills::sync_enabled_to_claude(&state.db, app_type) {
            warnings.push(format!("Skills sync warning: {}", e));
        }

        // (g) Update proxy target DB record
        if let Err(e) = state.db.set_proxy_target_provider_id(&id) {
            warnings.push(format!("Failed to update proxy target: {}", e));
        }
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
