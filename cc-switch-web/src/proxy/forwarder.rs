//! Request forwarder with OAuth token handling

use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    response::Response,
};
use cc_switch_lib::oauth::codex_oauth_auth::CodexOAuthManager;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::{ProxyConfig, ProxyStatus};

/// Forwarder handles proxying requests to OpenAI API with Codex OAuth auth
pub struct Forwarder {
    pub config: ProxyConfig,
    pub status: RwLock<ProxyStatus>,
    http_client: reqwest::Client,
}

impl Forwarder {
    pub fn new(config: ProxyConfig) -> Result<Self, String> {
        let mut builder = reqwest::Client::builder().use_rustls_tls();
        if let Some(proxy_url) = config.http_proxy_url.as_deref().filter(|value| !value.trim().is_empty()) {
            let proxy = reqwest::Proxy::all(proxy_url)
                .map_err(|e| format!("Invalid Codex HTTP proxy URL: {e}"))?;
            builder = builder.proxy(proxy);
        }
        let http_client = builder
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

        Ok(Self {
            config,
            status: RwLock::new(ProxyStatus::new()),
            http_client,
        })
    }

    pub async fn set_running(&self, running: bool, listen_addr: Option<std::net::SocketAddr>) {
        let mut status = self.status.write().await;
        status.running = running;
        status.listen_addr = listen_addr;
        if running && status.start_time.is_none() {
            status.start_time = Some(std::time::Instant::now());
        }
    }

    pub async fn get_status(&self) -> ProxyStatus {
        self.status.read().await.clone()
    }

    pub async fn increment_requests(&self) {
        let mut status = self.status.write().await;
        status.request_count += 1;
    }

    /// Forward an incoming request to OpenAI API with Codex OAuth auth
    pub async fn forward(
        &self,
        state: Arc<ProxyState>,
        req: Request,
    ) -> Result<Response, StatusCode> {
        // Increment request count
        self.increment_requests().await;

        // Get Codex OAuth token
        let codex_oauth = state.codex_oauth.clone();
        let token_result = match state.codex_account_id.as_deref() {
            Some(account_id) => codex_oauth.get_valid_token_for_account(account_id).await,
            None => codex_oauth.get_valid_token().await,
        };
        let token = match token_result {
            Ok(t) => t,
            Err(e) => {
                log::error!("[Proxy] Failed to get Codex OAuth token: {}", e);
                return Err(StatusCode::UNAUTHORIZED);
            }
        };

        // Get account ID for ChatGPT-Account-Id header
        let account_id = match state.codex_account_id.clone() {
            Some(account_id) => Some(account_id),
            None => codex_oauth.get_status().await.default_account_id,
        };

        // Build upstream URL
        let path = req.uri().path();
        let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
        let upstream_url = format!("{}{}{}", self.config.upstream_url, path, query);

        // Extract method and headers
        let method = req.method().clone();
        let headers = req.headers().clone();

        // Build request to upstream
        let mut upstream_req = reqwest::Request::new(method, upstream_url.parse().map_err(|_| StatusCode::BAD_REQUEST)?);

        // Set auth headers
        {
            let upstream_headers = upstream_req.headers_mut();
            upstream_headers.insert("authorization", format!("Bearer {}", token).parse().unwrap());
            upstream_headers.insert("originator", "cc-switch".parse().unwrap());
            if let Some(ref id) = account_id {
                upstream_headers.insert("chatgpt-account-id", id.parse().unwrap());
            }
        }

        // Copy other headers (excluding hop-by-hop)
        let hop_by_hop = [
            "connection", "keep-alive", "proxy-authenticate", "proxy-authorization",
            "te", "trailers", "transfer-encoding", "upgrade",
        ];
        for (name, value) in headers.iter() {
            let name_lower = name.as_str().to_lowercase();
            if !hop_by_hop.contains(&name_lower.as_str()) && !upstream_req.headers().contains_key(name) {
                upstream_req.headers_mut().insert(name, value.clone());
            }
        }

        // Send request
        let upstream_res = self.http_client.execute(upstream_req).await.map_err(|e| {
            log::error!("[Proxy] HTTP error: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

        // Convert response
        let status = StatusCode::from_u16(upstream_res.status().as_u16()).unwrap_or(StatusCode::OK);

        let mut response = Response::builder().status(status);
        for (name, value) in upstream_res.headers().iter() {
            response = response.header(name, value);
        }

        let body = upstream_res.bytes().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
        Ok(response.body(Body::from(body)).unwrap())
    }
}

/// Shared state for proxy server
#[derive(Clone)]
pub struct ProxyState {
    pub codex_oauth: Arc<CodexOAuthManager>,
    pub codex_account_id: Option<String>,
}

impl ProxyState {
    pub fn new(codex_oauth: Arc<CodexOAuthManager>, codex_account_id: Option<String>) -> Self {
        Self { codex_oauth, codex_account_id }
    }
}
