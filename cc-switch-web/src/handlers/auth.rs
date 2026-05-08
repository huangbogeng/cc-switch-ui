//! Auth handlers

use super::super::state::AppState;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct LoginRequest {
    token: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    success: bool,
    message: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Json<LoginResponse> {
    log::info!("[Auth] login requested");
    if payload.token == state.token {
        log::info!("[Auth] login success");
        Json(LoginResponse {
            success: true,
            message: "Login successful".to_string(),
        })
    } else {
        log::warn!("[Auth] login failed: invalid token");
        Json(LoginResponse {
            success: false,
            message: "Invalid token".to_string(),
        })
    }
}
