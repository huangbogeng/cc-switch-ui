//! Copilot OAuth handlers

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;
use cc_switch_lib::oauth::copilot::{CopilotAuthError, CopilotUsageResponse};

#[derive(Debug, Serialize)]
pub struct OAuthStatusResponse {
    pub authenticated: bool,
    pub accounts: Vec<GitHubAccountInfo>,
    pub default_account_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GitHubAccountInfo {
    pub id: String,
    pub login: String,
    pub avatar_url: Option<String>,
    pub github_domain: String,
}

#[derive(Debug, Deserialize)]
pub struct StartOAuthRequest {
    pub github_domain: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PollOAuthRequest {
    pub device_code: String,
    pub github_domain: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RemoveAccountRequest {
    pub account_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SetDefaultRequest {
    pub account_id: String,
}

#[derive(Debug, Serialize)]
pub struct StartOAuthResponse {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
}

#[derive(Debug, Serialize)]
pub struct PollOAuthResponse {
    pub success: bool,
    pub account: Option<GitHubAccountInfo>,
    pub error: Option<String>,
}

/// GET /api/copilot/oauth/status
pub async fn copilot_oauth_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let accounts = state.copilot_oauth.list_accounts().await;
    let default_account_id = state.copilot_oauth.get_default_account_id().await;

    let response = OAuthStatusResponse {
        authenticated: !accounts.is_empty(),
        accounts: accounts
            .into_iter()
            .map(|a| GitHubAccountInfo {
                id: a.id,
                login: a.login,
                avatar_url: a.avatar_url,
                github_domain: a.github_domain,
            })
            .collect(),
        default_account_id,
    };

    Json(response)
}

/// POST /api/copilot/oauth/start
pub async fn copilot_oauth_start(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StartOAuthRequest>,
) -> Result<Json<StartOAuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    match state
        .copilot_oauth
        .start_device_flow(payload.github_domain.as_deref())
        .await
    {
        Ok(device_code) => Ok(Json(StartOAuthResponse {
            user_code: device_code.user_code,
            verification_uri: device_code.verification_uri,
            expires_in: device_code.expires_in,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

/// POST /api/copilot/oauth/poll
pub async fn copilot_oauth_poll(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PollOAuthRequest>,
) -> impl IntoResponse {
    match state
        .copilot_oauth
        .poll_for_token(&payload.device_code, payload.github_domain.as_deref())
        .await
    {
        Ok(account) => Json(PollOAuthResponse {
            success: account.is_some(),
            account: account.map(|a| GitHubAccountInfo {
                id: a.id,
                login: a.login,
                avatar_url: a.avatar_url,
                github_domain: a.github_domain,
            }),
            error: None,
        }),
        Err(e) => Json(PollOAuthResponse {
            success: false,
            account: None,
            error: Some(e.to_string()),
        }),
    }
}

/// POST /api/copilot/oauth/remove
pub async fn copilot_oauth_remove(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RemoveAccountRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state
        .copilot_oauth
        .remove_account(&payload.account_id)
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({ "success": true }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

/// POST /api/copilot/oauth/set-default
pub async fn copilot_oauth_set_default(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetDefaultRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state
        .copilot_oauth
        .set_default_account(&payload.account_id)
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({ "success": true }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

/// GET /api/copilot/usage
pub async fn copilot_usage(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CopilotUsageResponse>, (StatusCode, Json<serde_json::Value>)> {
    match state.copilot_oauth.fetch_usage().await {
        Ok(usage) => Ok(Json(usage)),
        Err(e) => {
            let status = copilot_usage_error_status(&e);
            if status.is_server_error() {
                log::error!("[CopilotAuth] 获取使用量失败: {}", e);
            } else {
                log::debug!("[CopilotAuth] 使用量暂不可用: {}", e);
            }
            Err((status, Json(serde_json::json!({ "error": e.to_string() }))))
        }
    }
}

fn copilot_usage_error_status(error: &CopilotAuthError) -> StatusCode {
    match error {
        CopilotAuthError::AccountNotFound(_) | CopilotAuthError::GitHubTokenInvalid => {
            StatusCode::NOT_FOUND
        }
        CopilotAuthError::NoCopilotSubscription => StatusCode::FORBIDDEN,
        CopilotAuthError::InvalidDomain(_) => StatusCode::BAD_REQUEST,
        CopilotAuthError::NetworkError(_)
        | CopilotAuthError::CopilotTokenFetchFailed(_)
        | CopilotAuthError::ParseError(_) => StatusCode::BAD_GATEWAY,
        CopilotAuthError::IoError(_)
        | CopilotAuthError::DeviceFlowNotStarted
        | CopilotAuthError::AuthorizationPending
        | CopilotAuthError::AccessDenied
        | CopilotAuthError::ExpiredToken => StatusCode::BAD_REQUEST,
    }
}
