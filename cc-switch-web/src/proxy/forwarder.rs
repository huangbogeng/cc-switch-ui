//! Request forwarder with OAuth token handling

use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::{header, StatusCode},
    response::Response,
};
use cc_switch_lib::oauth::codex_oauth_auth::CodexOAuthManager;
use reqwest::Method;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::headers::{add_codex_session_headers, copy_forward_headers, copy_response_headers};
use super::responses_aggregate::responses_sse_to_anthropic_message;
use super::session::{build_prompt_cache_key, extract_session_id};
use super::streaming_responses::responses_sse_to_anthropic;
use super::transform_responses::anthropic_to_codex_responses;
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
        if let Some(proxy_url) = config
            .http_proxy_url
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
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

        // Build upstream URL. Codex OAuth is always routed to the ChatGPT Codex
        // Responses endpoint; the incoming Claude endpoint is only used for logs.
        let path = req.uri().path().to_string();
        let query = req.uri().query().unwrap_or_default().to_string();
        let upstream_url = self.config.upstream_url.clone();

        // Extract method and headers
        let method = req.method().clone();
        let headers = req.headers().clone();
        let body = to_bytes(req.into_body(), 50 * 1024 * 1024)
            .await
            .map_err(|e| {
                log::error!("[Proxy] Failed to read request body for {}: {}", path, e);
                StatusCode::BAD_REQUEST
            })?;
        let mut body_json: Value = serde_json::from_slice(&body).map_err(|e| {
            log::error!("[Proxy] Failed to parse request body for {}: {}", path, e);
            StatusCode::BAD_REQUEST
        })?;
        apply_model_mapping(&mut body_json, &self.config.model_mapping);
        let requested_stream = body_json.get("stream").and_then(Value::as_bool) != Some(false);
        let session = extract_session_id(&headers, &body_json);
        let prompt_cache_key = build_prompt_cache_key(
            self.config.prompt_cache_key.as_deref(),
            session.as_ref().map(|result| result.session_id.as_str()),
            &self.config.prompt_cache_key_fallback,
        );
        log::info!(
            "[Proxy] session_source={:?}, prompt_cache_key_hash={} (len={})",
            session.as_ref().map(|session| session.source),
            cache_key_log_id(&prompt_cache_key),
            prompt_cache_key.chars().count()
        );
        let request_json = match anthropic_to_codex_responses(
            body_json,
            Some(&prompt_cache_key),
            self.config.codex_fast_mode,
        ) {
            Ok(value) => value,
            Err(e) => {
                log::error!("[Proxy] Failed to convert request body for {}: {}", path, e);
                return Err(StatusCode::BAD_REQUEST);
            }
        };

        log::info!(
            "[Proxy] {} {}{} -> {}{}",
            method,
            path,
            if query.is_empty() {
                String::new()
            } else {
                format!("?{query}")
            },
            upstream_url,
            if self.config.http_proxy_url.is_some() {
                " via configured proxy"
            } else {
                ""
            }
        );

        let reqwest_method =
            Method::from_bytes(method.as_str().as_bytes()).map_err(|_| StatusCode::BAD_REQUEST)?;
        let request_body = serde_json::to_vec(&request_json).map_err(|e| {
            log::error!(
                "[Proxy] Failed to serialize request body for {}: {}",
                path,
                e
            );
            StatusCode::BAD_REQUEST
        })?;
        let mut upstream_req = self
            .http_client
            .request(reqwest_method, &upstream_url)
            .body(request_body);

        // Set auth headers
        upstream_req = upstream_req
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .header("originator", "cc-switch")
            .header(header::ACCEPT, "text/event-stream")
            .header(header::ACCEPT_ENCODING, "identity");
        if let Some(ref id) = account_id {
            upstream_req = upstream_req.header("chatgpt-account-id", id);
        }
        if let Some(session) = session.as_ref() {
            upstream_req = add_codex_session_headers(upstream_req, &session.session_id);
        }

        // Copy other headers (excluding hop-by-hop)
        upstream_req = copy_forward_headers(upstream_req, &headers);
        upstream_req = upstream_req.header(header::CONTENT_TYPE, "application/json");

        // Send request
        let upstream_res = upstream_req.send().await.map_err(|e| {
            log::error!("[Proxy] HTTP error: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

        // Convert response
        let status = StatusCode::from_u16(upstream_res.status().as_u16()).unwrap_or(StatusCode::OK);
        log::info!("[Proxy] Upstream status for {}: {}", path, status);

        let mut response = Response::builder().status(status);
        response = copy_response_headers(response, upstream_res.headers());

        if !status.is_success() {
            let body = upstream_res.bytes().await.map_err(|e| {
                log::error!("[Proxy] Failed to read upstream error body: {}", e);
                StatusCode::BAD_GATEWAY
            })?;
            let excerpt = String::from_utf8_lossy(&body);
            log::error!(
                "[Proxy] Upstream error for {}: {}",
                path,
                excerpt.chars().take(1000).collect::<String>()
            );
            return Ok(response.body(Body::from(body)).unwrap());
        }

        if requested_stream {
            let stream = responses_sse_to_anthropic(upstream_res.bytes_stream());
            return Ok(response
                .header(header::CONTENT_TYPE, "text/event-stream")
                .body(Body::from_stream(stream))
                .unwrap());
        }

        let body = upstream_res.bytes().await.map_err(|e| {
            log::error!("[Proxy] Failed to read upstream response body: {}", e);
            StatusCode::BAD_GATEWAY
        })?;
        let message = responses_sse_to_anthropic_message(&body).map_err(|e| {
            log::error!("[Proxy] Failed to aggregate Responses SSE: {}", e);
            StatusCode::BAD_GATEWAY
        })?;
        Ok(response
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(message.to_string()))
            .unwrap())
    }
}

fn apply_model_mapping(body: &mut Value, mapping: &super::types::ModelMapping) {
    let Some(original) = body.get("model").and_then(Value::as_str) else {
        return;
    };
    let mapped = mapping.map_model(original);
    if mapped != original {
        log::debug!("[Proxy] Model mapping: {} -> {}", original, mapped);
        body["model"] = json!(mapped);
    }
}

fn cache_key_log_id(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Shared state for proxy server
#[derive(Clone)]
pub struct ProxyState {
    pub codex_oauth: Arc<CodexOAuthManager>,
    pub codex_account_id: Option<String>,
}

impl ProxyState {
    pub fn new(codex_oauth: Arc<CodexOAuthManager>, codex_account_id: Option<String>) -> Self {
        Self {
            codex_oauth,
            codex_account_id,
        }
    }
}
