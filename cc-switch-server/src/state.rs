//! Application state shared across all handlers

use super::proxy::ProxyServer;
use cc_switch_lib::database::Database;
use cc_switch_lib::oauth::codex::CodexOAuthManager;
use cc_switch_lib::oauth::copilot::CopilotAuthManager;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub codex_oauth: Arc<CodexOAuthManager>,
    pub copilot_oauth: Arc<CopilotAuthManager>,
    pub token: String,
    pub proxy_server: Arc<RwLock<Option<ProxyServer>>>,
    pub proxy_listen_port: u16,
    pub db: Arc<Database>,
}
