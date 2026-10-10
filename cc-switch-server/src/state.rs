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
    /// Host the local proxy binds to. Defaults to loopback; follows the admin
    /// `--host` opt-in. The proxy has no auth layer, so exposing it beyond
    /// loopback hands stored provider credentials to anyone who can reach it.
    pub proxy_host: String,
    pub proxy_listen_port: u16,
    pub db: Arc<Database>,
}

// Serialize actions that touch provider references and the live takeover file.
pub(crate) static PROVIDER_MUTATIONS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(test)]
pub(crate) struct TestHome {
    pub path: std::path::PathBuf,
    previous: Option<std::ffi::OsString>,
}

#[cfg(test)]
impl TestHome {
    pub fn new() -> Self {
        let home = Self {
            path: std::env::temp_dir().join(format!("cc-server-test-{}", rand::random::<u64>())),
            previous: std::env::var_os("CC_SWITCH_TEST_HOME"),
        };
        std::fs::create_dir_all(&home.path).unwrap();
        unsafe {
            std::env::set_var("CC_SWITCH_TEST_HOME", &home.path);
        }
        cc_switch_lib::settings::reload_settings().unwrap();
        home
    }
}

#[cfg(test)]
impl Drop for TestHome {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe {
                std::env::set_var("CC_SWITCH_TEST_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("CC_SWITCH_TEST_HOME");
            },
        }
        cc_switch_lib::settings::reload_settings().unwrap();
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
