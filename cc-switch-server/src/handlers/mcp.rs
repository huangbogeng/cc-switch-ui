//! MCP server management handlers

use super::super::state::AppState;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use cc_switch_lib::database::McpServerRecord;
use serde_json::json;
use std::sync::Arc;

const APP_TYPE: &str = "claude_code";

pub async fn list_mcp_servers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.get_all_mcp_servers(APP_TYPE) {
        Ok(servers) => {
            Json(json!({ "servers": servers })).into_response()
        }
        Err(e) => {
            log::error!("Failed to list MCP servers: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn save_mcp_server(
    State(state): State<Arc<AppState>>,
    Json(body): Json<McpServerRecord>,
) -> impl IntoResponse {
    let mut server = body;
    server.app_type = APP_TYPE.to_string();

    log::info!("[MCP] save_mcp_server id={} name={}", server.id, server.name);
    match state.db.save_mcp_server(&server) {
        Ok(()) => Json(json!({ "success": true })).into_response(),
        Err(e) => {
            log::error!("Failed to save MCP server: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn delete_mcp_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    log::info!("[MCP] delete_mcp_server id={}", id);
    match state.db.delete_mcp_server(&id, APP_TYPE) {
        Ok(true) => Json(json!({ "success": true })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "MCP server not found"})),
        )
            .into_response(),
        Err(e) => {
            log::error!("Failed to delete MCP server: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn import_mcp_servers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[MCP] Import from ~/.claude.json requested");
    match cc_switch_lib::mcp::import_from_claude(&state.db, APP_TYPE) {
        Ok(count) => Json(json!({ "success": true, "imported": count })).into_response(),
        Err(e) => {
            log::error!("Failed to import MCP servers: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn toggle_mcp_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    log::info!("[MCP] Toggle enabled for id={}", id);
    match state.db.get_all_mcp_servers(APP_TYPE) {
        Ok(servers) => {
            if let Some(server) = servers.into_iter().find(|s| s.id == id) {
                let mut toggled = server.clone();
                toggled.enabled = !toggled.enabled;
                match state.db.save_mcp_server(&toggled) {
                    Ok(()) => Json(json!({ "success": true, "enabled": toggled.enabled })).into_response(),
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": e.to_string()})),
                    ).into_response(),
                }
            } else {
                (StatusCode::NOT_FOUND, Json(json!({"error": "MCP server not found"}))).into_response()
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ).into_response(),
    }
}

pub async fn sync_mcp_servers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[MCP] Manual sync requested");
    match cc_switch_lib::mcp::sync_enabled_to_claude(&state.db, APP_TYPE) {
        Ok(()) => Json(json!({ "success": true })).into_response(),
        Err(e) => {
            log::error!("Failed to sync MCP servers: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}
