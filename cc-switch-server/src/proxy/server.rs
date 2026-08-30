//! Proxy server implementation

use super::failover_switch::FailoverSwitchManager;
use super::forwarder::{ForwardResult, Forwarder};
use super::handlers::provider_codex_account_id;
use super::provider_router::{ProviderRouter, SelectProvidersError};
use super::types::ProxyConfig;
use super::types::ProxyState;
use axum::{
    body::Body,
    extract::Request,
    response::Response,
    routing::{get, post},
    Router,
};
use cc_switch_lib::database::Database;
use cc_switch_lib::error::AppError;
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
        registry: Arc<ProviderRegistry>,
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
        let runtime_state = Arc::new(ProxyRuntimeState {
            db: db.clone(),
            provider_hint_id: provider_id,
            app_type: app_type.to_string(),
            codex_account_id,
            registry,
            provider_router: Arc::new(RwLock::new(ProviderRouter::with_database(
                false,
                db.clone(),
            ))),
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

    /// Check whether the proxy server is currently running.
    pub async fn is_running(&self) -> bool {
        self.server_task.read().await.is_some()
    }

    /// Hot-switch the proxy's active target provider.
    ///
    /// Updates the database route target so subsequent requests are forwarded
    /// to the new provider. This is the core of proxy takeover mode — the live
    /// config already points at the proxy (127.0.0.1:15721), so we only need
    /// to change which provider the proxy routes to.
    pub async fn hot_switch_provider(
        &self,
        db: &Database,
        provider_id: &str,
    ) -> Result<(), AppError> {
        db.set_proxy_target_provider_id(provider_id)?;
        log::info!("[Proxy] Hot-switched target provider to '{}'", provider_id);
        Ok(())
    }
}

fn build_proxy_router(forwarder: Arc<Forwarder>, runtime_state: Arc<ProxyRuntimeState>) -> Router {
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
    let method = req.method().clone();
    let uri = req.uri().clone();
    log::info!(
        "[Proxy] >>> {} {} | headers: {:?}",
        method,
        uri,
        req.headers()
            .iter()
            .map(|(k, v)| format!("{k:?}: {:?}", v.to_str().unwrap_or("(binary)")))
            .collect::<Vec<_>>()
    );

    let current_provider = match resolve_current_provider(&runtime_state) {
        Ok(provider) => {
            log::info!(
                "[Proxy] resolved provider: id={} name={}",
                provider.id,
                provider.name
            );
            provider
        }
        Err(status) => {
            log::error!("[Proxy] resolve_current_provider failed status={}", status);
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
    let candidate_result = {
        let mut router = runtime_state.provider_router.write().await;
        router.set_auto_failover_enabled(auto_failover_enabled);
        router.select_providers(&runtime_state.app_type, &current_provider, &all_providers)
    };
    let provider_candidates = match candidate_result {
        Ok(candidates) => candidates,
        Err(SelectProvidersError::AllCandidatesCircuitOpen) => {
            return Response::builder()
                .status(axum::http::StatusCode::SERVICE_UNAVAILABLE)
                .body(Body::empty())
                .unwrap();
        }
    };

    log::info!(
        "[Proxy] {} provider candidate(s) after circuit-breaker filter",
        provider_candidates.len()
    );

    let proxy_states: Vec<_> = provider_candidates
        .into_iter()
        .filter_map(|provider| {
            let adapter = runtime_state.registry.find_for_provider(&provider);
            match adapter {
                Some(adapter) => {
                    log::info!(
                        "[Proxy] adapter matched: provider={} type={}",
                        provider.id,
                        adapter.provider_type()
                    );
                    let provider_id = provider.id.clone();
                    Some(Arc::new(ProxyState::new(
                        adapter,
                        provider_codex_account_id(&provider)
                            .or_else(|| runtime_state.codex_account_id.clone()),
                        provider,
                        runtime_state.db.clone(),
                        provider_id,
                    )))
                }
                None => {
                    log::error!(
                        "[Proxy] NO adapter for provider={} provider_type={:?}",
                        provider.id,
                        provider.meta.get("providerType").and_then(|v| v.as_str())
                    );
                    None
                }
            }
        })
        .collect();

    if proxy_states.is_empty() {
        log::error!("[Proxy] no adapter matched any candidate — returning 502");
        return Response::builder()
            .status(axum::http::StatusCode::BAD_GATEWAY)
            .body(Body::empty())
            .unwrap();
    }

    log::info!(
        "[Proxy] forwarding with {} proxy state(s) | app_type={}",
        proxy_states.len(),
        runtime_state.app_type
    );

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
                log::info!(
                    "[Proxy] <<< success provider={provider_id} status={}",
                    response.status()
                );
                response
            },
        )
        .unwrap_or_else(|status| {
            log::error!("[Proxy] <<< all attempts failed status={}", status);
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

#[cfg(test)]
mod tests {
    use axum::routing::post;
    use axum::Router;

    #[test]
    fn proxy_accepts_openai_chat_paths() {
        let _app: Router<()> = Router::new()
            .route(
                "/chat/completions",
                post(|| async { axum::http::StatusCode::OK }),
            )
            .route(
                "/v1/chat/completions",
                post(|| async { axum::http::StatusCode::OK }),
            );
    }

    #[test]
    fn proxy_accepts_responses_paths() {
        let _app: Router<()> = Router::new()
            .route("/responses", post(|| async { axum::http::StatusCode::OK }))
            .route(
                "/v1/responses",
                post(|| async { axum::http::StatusCode::OK }),
            );
    }
}
