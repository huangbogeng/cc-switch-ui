//! Proxy server implementation

use super::adapters::create_registry;
use super::forwarder::{Forwarder, ProxyState};
use super::types::ProxyConfig;
use axum::{body::Body, extract::Request, response::Response, routing::get, Router};
use cc_switch_lib::database::Database;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

/// Proxy server that can be started/stopped
pub struct ProxyServer {
    config: ProxyConfig,
    shutdown_tx: RwLock<Option<tokio::sync::oneshot::Sender<()>>>,
    server_task: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl ProxyServer {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            shutdown_tx: RwLock::new(None),
            server_task: RwLock::new(None),
        }
    }

    /// Start the proxy server
    pub async fn start(
        &self,
        codex_oauth: Arc<cc_switch_lib::oauth::codex::CodexOAuthManager>,
        copilot_auth: Arc<cc_switch_lib::oauth::copilot::CopilotAuthManager>,
        codex_account_id: Option<String>,
        db: Arc<Database>,
        provider_id: String,
        app_type: &str,
    ) -> Result<SocketAddr, String> {
        // Check if already running
        if self.server_task.read().await.is_some() {
            return Err("Proxy already running".to_string());
        }

        // Get provider from database
        let provider = db
            .get_provider(&provider_id, app_type)
            .map_err(|e| format!("Failed to get provider: {}", e))?
            .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

        // Create registry and find adapter
        let registry = create_registry(codex_oauth.clone(), copilot_auth);
        let adapter = registry
            .find_for_provider(&provider)
            .ok_or_else(|| "No adapter found for provider type".to_string())?;

        let forwarder = Arc::new(Forwarder::new(self.config.clone())?);
        let proxy_state = Arc::new(ProxyState::new(
            adapter,
            codex_account_id,
            provider,
            db,
            provider_id,
        ));
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let app = Router::new()
            .route("/v1/*axum", get(handle_proxy).post(handle_proxy))
            .route("/health", get(|| async { "ok" }))
            .with_state((forwarder.clone(), proxy_state))
            .layer(TraceLayer::new_for_http());

        let listener = TcpListener::bind(self.config.listen_addr)
            .await
            .map_err(|e| e.to_string())?;
        let actual_addr = listener.local_addr().map_err(|e| e.to_string())?;

        // Set running status
        forwarder.set_running(true, Some(actual_addr)).await;

        // Spawn server task
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .ok();
        });

        *self.shutdown_tx.write().await = Some(shutdown_tx);
        *self.server_task.write().await = Some(task);

        log::info!("[Proxy] Server started on {}", actual_addr);
        Ok(actual_addr)
    }

    /// Stop the proxy server
    pub async fn stop(&self) -> Result<(), String> {
        let tx = self.shutdown_tx.write().await.take();
        if let Some(tx) = tx {
            tx.send(()).map_err(|_| "Already stopped".to_string())?;
        }

        if let Some(task) = self.server_task.write().await.take() {
            task.await.map_err(|e| e.to_string())?;
        }

        log::info!("[Proxy] Server stopped");
        Ok(())
    }
}

/// Handle proxy requests
async fn handle_proxy(
    axum::extract::State((forwarder, proxy_state)): axum::extract::State<(
        Arc<Forwarder>,
        Arc<ProxyState>,
    )>,
    req: Request,
) -> Response {
    forwarder
        .forward(proxy_state, req)
        .await
        .unwrap_or_else(|status| {
            Response::builder()
                .status(status)
                .body(Body::empty())
                .unwrap()
        })
}
