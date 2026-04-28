//! Auth handlers

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use super::super::state::AppState;

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
    if payload.token == state.token {
        Json(LoginResponse {
            success: true,
            message: "Login successful".to_string(),
        })
    } else {
        Json(LoginResponse {
            success: false,
            message: "Invalid token".to_string(),
        })
    }
}
