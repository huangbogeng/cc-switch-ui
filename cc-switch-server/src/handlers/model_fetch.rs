use crate::state::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{http::StatusCode, Json};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexModelsRequest {
    pub account_id: Option<String>,
}

pub async fn fetch_codex_models(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CodexModelsRequest>,
) -> impl IntoResponse {
    match state
        .codex_oauth
        .fetch_models(payload.account_id.as_deref())
        .await
    {
        Ok(models) => Json(serde_json::json!({ "models": models })).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchModelsRequest {
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub is_full_url: bool,
    pub models_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectEndpointTypeRequest {
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub is_full_url: bool,
}

pub async fn fetch_models_for_config(Json(payload): Json<FetchModelsRequest>) -> impl IntoResponse {
    match cc_switch_lib::providers::fetch_models(
        &payload.base_url,
        &payload.api_key,
        payload.is_full_url,
        payload.models_url.as_deref(),
    )
    .await
    {
        Ok(models) => Json(serde_json::json!({ "models": models })).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error })),
        )
            .into_response(),
    }
}

pub async fn detect_endpoint_type(
    Json(payload): Json<DetectEndpointTypeRequest>,
) -> impl IntoResponse {
    match cc_switch_lib::providers::detect_endpoint_type(
        &payload.base_url,
        &payload.api_key,
        payload.is_full_url,
    )
    .await
    {
        Ok(result) => Json(serde_json::json!(result)).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error })),
        )
            .into_response(),
    }
}
