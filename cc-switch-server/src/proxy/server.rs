//! Proxy server implementation

use super::adapters::create_registry;
use super::failover_switch::FailoverSwitchManager;
use super::forwarder::{ForwardResult, Forwarder, ProxyState};
use super::provider_router::{ProviderRouter, SelectProvidersError};
use super::types::ProxyConfig;
use axum::{
    body::Body,
    extract::Request,
    response::Response,
    routing::{get, post},
    Router,
};
use cc_switch_lib::database::Database;
use cc_switch_lib::providers::ProviderRegistry;
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

#[derive(Clone)]
struct ProxyRuntimeState {
    db: Arc<Database>,
    provider_hint_id: String,
    app_type: String,
    codex_account_id: Option<String>,
    registry: Arc<ProviderRegistry>,
    provider_router: Arc<RwLock<ProviderRouter>>,
    failover_switch: Arc<FailoverSwitchManager>,
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

        let forwarder = Arc::new(Forwarder::new(self.config.clone())?);
        let registry = Arc::new(create_registry(codex_oauth.clone(), copilot_auth));
        let runtime_state = Arc::new(ProxyRuntimeState {
            db: db.clone(),
            provider_hint_id: provider_id,
            app_type: app_type.to_string(),
            codex_account_id,
            registry,
            provider_router: Arc::new(RwLock::new(ProviderRouter::new(false))),
            failover_switch: Arc::new(FailoverSwitchManager::new(db.clone())),
        });
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let app = build_proxy_router(forwarder.clone(), runtime_state);

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

fn build_proxy_router(
    forwarder: Arc<Forwarder>,
    runtime_state: Arc<ProxyRuntimeState>,
) -> Router {
    Router::new()
        .route("/v1/*axum", get(handle_proxy).post(handle_proxy))
        .route("/chat/completions", post(handle_proxy))
        .route("/v1/chat/completions", post(handle_proxy))
        .route("/responses", post(handle_proxy))
        .route("/v1/responses", post(handle_proxy))
        .route("/health", get(|| async { "ok" }))
        .with_state((forwarder, runtime_state))
        .layer(TraceLayer::new_for_http())
}

/// Handle proxy requests
async fn handle_proxy(
    axum::extract::State((forwarder, runtime_state)): axum::extract::State<(
        Arc<Forwarder>,
        Arc<ProxyRuntimeState>,
    )>,
    req: Request,
) -> Response {
    let current_provider = match resolve_current_provider(&runtime_state) {
        Ok(provider) => provider,
        Err(status) => {
            return Response::builder()
                .status(status)
                .body(Body::empty())
                .unwrap();
        }
    };

    let all_providers = match runtime_state.db.list_providers(&runtime_state.app_type) {
        Ok(providers) => providers,
        Err(e) => {
            log::error!("[Proxy] Failed to list providers: {}", e);
            return Response::builder()
                .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap();
        }
    };

    let auto_failover_enabled = runtime_state
        .db
        .get_proxy_config()
        .ok()
        .flatten()
        .map(|cfg| cfg.auto_failover_enabled)
        .unwrap_or_else(|| {
            current_provider
                .meta
                .get("auto_failover_enabled")
                .or_else(|| current_provider.meta.get("autoFailoverEnabled"))
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        });
    let provider_candidates = match {
        let mut router = runtime_state.provider_router.write().await;
        router.set_auto_failover_enabled(auto_failover_enabled);
        router.select_providers(&runtime_state.app_type, &current_provider, &all_providers)
    } {
        Ok(candidates) => candidates,
        Err(SelectProvidersError::AllCandidatesCircuitOpen) => {
            return Response::builder()
                .status(axum::http::StatusCode::SERVICE_UNAVAILABLE)
                .body(Body::empty())
                .unwrap();
        }
    };

    let proxy_states = provider_candidates
        .into_iter()
        .filter_map(|provider| {
            let adapter = runtime_state.registry.find_for_provider(&provider)?;
            let provider_id = provider.id.clone();
            Some(Arc::new(ProxyState::new(
                adapter,
                provider_codex_account_id(&provider)
                    .or_else(|| runtime_state.codex_account_id.clone()),
                provider,
                runtime_state.db.clone(),
                provider_id,
            )))
        })
        .collect::<Vec<_>>();

    forwarder
        .forward_with_retry(
            &runtime_state.app_type,
            runtime_state.provider_router.clone(),
            runtime_state.failover_switch.clone(),
            current_provider.id.clone(),
            proxy_states,
            req,
        )
        .await
        .map(
            |ForwardResult {
                 response,
                 provider_id,
             }| {
                log::info!("[Proxy] Request succeeded via provider={provider_id}");
                response
            },
        )
        .unwrap_or_else(|status| {
            Response::builder()
                .status(status)
                .body(Body::empty())
                .unwrap()
        })
}

fn resolve_current_provider(
    runtime_state: &ProxyRuntimeState,
) -> Result<cc_switch_lib::database::Provider, axum::http::StatusCode> {
    let target_id = runtime_state
        .db
        .get_proxy_target_provider_id()
        .ok()
        .flatten()
        .or_else(|| {
            runtime_state
                .db
                .get_current_provider_id(&runtime_state.app_type)
                .ok()
                .flatten()
        })
        .unwrap_or_else(|| runtime_state.provider_hint_id.clone());

    runtime_state
        .db
        .get_provider(&target_id, &runtime_state.app_type)
        .ok()
        .flatten()
        .ok_or(axum::http::StatusCode::BAD_GATEWAY)
}

fn provider_codex_account_id(provider: &cc_switch_lib::database::Provider) -> Option<String> {
    provider
        .meta
        .get("authBinding")
        .and_then(|value| value.get("accountId"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

#[cfg(test)]
mod tests {
    use axum::routing::post;
    use axum::Router;

    #[test]
    fn proxy_accepts_openai_chat_paths() {
        let _app: Router<()> = Router::new()
            .route("/chat/completions", post(|| async { axum::http::StatusCode::OK }))
            .route(
                "/v1/chat/completions",
                post(|| async { axum::http::StatusCode::OK }),
            );
    }

    #[test]
    fn proxy_accepts_responses_paths() {
        let _app: Router<()> = Router::new()
            .route("/responses", post(|| async { axum::http::StatusCode::OK }))
            .route("/v1/responses", post(|| async { axum::http::StatusCode::OK }));
    }
}
