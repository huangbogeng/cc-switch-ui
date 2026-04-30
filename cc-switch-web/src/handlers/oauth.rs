//! OAuth handlers for Codex

use super::super::state::AppState;
use axum::response::IntoResponse;
use axum::{extract::State, http::StatusCode, Json};
use cc_switch_lib::oauth::codex_oauth_auth::CodexOAuthError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
pub struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

pub async fn codex_oauth_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let status = state.codex_oauth.get_status().await;
    let default_id = status.default_account_id.as_deref();
    Json(serde_json::json!({
        "authenticated": status.authenticated,
        "accounts": status.accounts.iter().map(|a| {
            serde_json::json!({
                "id": a.id,
                "login": a.login,
                "is_default": Some(a.id.as_str()) == default_id
            })
        }).collect::<Vec<_>>()
    }))
}

pub async fn codex_oauth_start(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.codex_oauth.start_device_flow().await {
        Ok(resp) => Json(DeviceCodeResponse {
            device_code: resp.device_code,
            user_code: resp.user_code,
            verification_uri: resp.verification_uri,
            expires_in: resp.expires_in,
            interval: resp.interval,
        })
        .into_response(),
        Err(e) => {
            log::error!("Failed to start OAuth: {}", e);
            let status = match &e {
                CodexOAuthError::NetworkError(message)
                    if message.contains("unsupported_country_region_territory")
                        || message.contains("request_forbidden") =>
                {
                    StatusCode::FORBIDDEN
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, Json(serde_json::json!({"error": format!("{}", e)}))).into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct PollRequest {
    device_code: String,
}

#[derive(Deserialize)]
pub struct AccountRequest {
    account_id: String,
}

pub async fn codex_oauth_poll(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PollRequest>,
) -> impl IntoResponse {
    match state.codex_oauth.poll_for_token(&payload.device_code).await {
        Ok(Some(account)) => {
            let status = state.codex_oauth.get_status().await;
            let default_id = status.default_account_id.as_deref();
            let account_id_str = account.id.as_str();
            Json(serde_json::json!({
                "success": true,
                "account": {
                    "id": account.id,
                    "login": account.login,
                    "is_default": Some(account_id_str) == default_id
                }
            }))
            .into_response()
        }
        Ok(None) => Json(serde_json::json!({"pending": true})).into_response(),
        Err(CodexOAuthError::AuthorizationPending) => {
            Json(serde_json::json!({"pending": true})).into_response()
        }
        Err(e) => {
            log::error!("OAuth poll error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    }
}

pub async fn codex_oauth_remove(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AccountRequest>,
) -> impl IntoResponse {
    match state.codex_oauth.remove_account(&payload.account_id).await {
        Ok(()) => Json(serde_json::json!({"success": true})).into_response(),
        Err(e) => {
            log::error!("Codex OAuth remove error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    }
}

pub async fn codex_oauth_set_default(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AccountRequest>,
) -> impl IntoResponse {
    match state
        .codex_oauth
        .set_default_account(&payload.account_id)
        .await
    {
        Ok(()) => Json(serde_json::json!({"success": true})).into_response(),
        Err(e) => {
            log::error!("Codex OAuth set default error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    }
}
