//! Application state shared across all handlers

use std::sync::Arc;
use tokio::sync::RwLock;
use cc_switch_lib::database::Database;
use cc_switch_lib::oauth::codex_oauth_auth::CodexOAuthManager;
use super::proxy::ProxyServer;

#[derive(Clone)]
pub struct AppState {
    pub codex_oauth: Arc<CodexOAuthManager>,
    pub token: String,
    pub proxy_server: Arc<RwLock<Option<ProxyServer>>>,
    pub proxy_listen_port: u16,
    pub db: Arc<Database>,
}
