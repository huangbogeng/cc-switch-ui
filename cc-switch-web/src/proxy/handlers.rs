//! Proxy handlers

use std::net::SocketAddr;
use std::sync::Arc;
use axum::{extract::State, Json};
use axum::response::IntoResponse;
use super::{ProxyConfig, ProxyServer};
use super::super::state::AppState;

pub async fn proxy_start(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if state.proxy_server.read().await.is_some() {
        return Json(serde_json::json!({"success": false, "error": "Proxy already running"})).into_response();
    }

    let status = state.codex_oauth.get_status().await;
    if !status.authenticated {
        return Json(serde_json::json!({"success": false, "error": "Not authenticated. Please complete OAuth first."})).into_response();
    }

    let proxy_addr = SocketAddr::from(([0, 0, 0, 0], state.proxy_listen_port));
    let config = ProxyConfig {
        listen_addr: proxy_addr,
        upstream_url: "https://api.openai.com".to_string(),
    };
    let server = ProxyServer::new(config);
    let listen_port = state.proxy_listen_port;

    match server.start(state.codex_oauth.clone()).await {
        Ok(_actual_addr) => {
            *state.proxy_server.write().await = Some(server);
            Json(serde_json::json!({"success": true, "listen_addr": format!("http://0.0.0.0:{}", listen_port), "message": "Proxy started"})).into_response()
        }
        Err(e) => Json(serde_json::json!({"success": false, "error": e})).into_response()
    }
}

pub async fn proxy_stop(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let server = state.proxy_server.write().await.take();
    match server {
        Some(s) => {
            if let Err(e) = s.stop().await {
                Json(serde_json::json!({"success": false, "error": e})).into_response()
            } else {
                Json(serde_json::json!({"success": true, "message": "Proxy stopped"})).into_response()
            }
        }
        None => Json(serde_json::json!({"success": false, "error": "Proxy not running"})).into_response()
    }
}

pub async fn proxy_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let running = state.proxy_server.read().await.is_some();
    Json(serde_json::json!({
        "running": running,
        "listen_addr": if running { Some(format!("http://0.0.0.0:{}", state.proxy_listen_port)) } else { None },
        "upstream_url": "https://api.openai.com"
    })).into_response()
}
