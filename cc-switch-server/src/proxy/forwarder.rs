//! Request forwarder with OAuth token handling

use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::{header, StatusCode},
    response::Response,
};
use cc_switch_lib::providers::{
    AuthStrategy, ProviderAdapter, StreamingResponseFormat, TransformInput,
};
use futures::StreamExt;
use reqwest::Method;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use super::failover_switch::FailoverSwitchManager;
use super::headers::{copy_forward_headers, copy_response_headers};
use super::provider_router::ProviderRouter;
use super::session::{build_prompt_cache_key, extract_session_id};
use super::streaming_responses::{
    openai_chat_sse_to_anthropic_with_usage, responses_sse_to_anthropic,
};
use super::types::{ProxyConfig, ProxyStatus};

/// Forwarder handles proxying requests to OpenAI API with Codex OAuth auth
pub struct Forwarder {
    pub config: ProxyConfig,
    pub status: RwLock<ProxyStatus>,
    http_client: reqwest::Client,
}

pub struct ForwardResult {
    pub response: Response,
    pub provider_id: String,
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
                .map_err(|e| format!("Invalid outbound network proxy URL: {e}"))?;
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

    #[allow(dead_code)]
    pub async fn get_status(&self) -> ProxyStatus {
        self.status.read().await.clone()
    }

    pub async fn increment_requests(&self) {
        let mut status = self.status.write().await;
        status.request_count += 1;
    }

    /// Forward an incoming request using provider candidates with retry/failover.
    pub async fn forward_with_retry(
        &self,
        app_type: &str,
        router: Arc<RwLock<ProviderRouter>>,
        failover_switch: Arc<FailoverSwitchManager>,
        current_provider_id: String,
        candidate_states: Vec<Arc<ProxyState>>,
        req: Request,
    ) -> Result<ForwardResult, StatusCode> {
        let started_at = Instant::now();
        if candidate_states.is_empty() {
            log::error!(
                "[Proxy] app_type={} current_provider={} no candidate providers available",
                app_type,
                current_provider_id
            );
            return Err(StatusCode::BAD_GATEWAY);
        }

        // Increment request count
        self.increment_requests().await;

        let path = req.uri().path().to_string();
        let query = req.uri().query().unwrap_or_default().to_string();
        let method = req.method().clone();
        let headers = req.headers().clone();
        let body = to_bytes(req.into_body(), 50 * 1024 * 1024)
            .await
            .map_err(|e| {
                log::error!("[Proxy] Failed to read request body for {}: {}", path, e);
                StatusCode::BAD_REQUEST
            })?;

        let mut last_response: Option<(String, Response)> = None;
        let mut last_status: Option<StatusCode> = None;
        log::info!(
            "[Proxy] app_type={} path={} method={} current_provider={} candidate_count={}",
            app_type,
            path,
            method,
            current_provider_id,
            candidate_states.len()
        );

        for state in candidate_states {
            let allowed = router
                .write()
                .await
                .allow_provider_request(app_type, &state.provider_id);
            if !allowed {
                log::warn!(
                    "[Proxy] app_type={} provider={} skipped: circuit breaker open",
                    app_type,
                    state.provider_id
                );
                continue;
            }
            log::info!(
                "[Proxy] app_type={} provider={} attempting request path={}",
                app_type,
                state.provider_id,
                path
            );

            let mut body_json: Value = serde_json::from_slice(&body).map_err(|e| {
                log::error!("[Proxy] Failed to parse request body for {}: {}", path, e);
                StatusCode::BAD_REQUEST
            })?;
            apply_model_mapping(&mut body_json, &self.config.model_mapping);
            let request_model = extract_request_model(&body_json);

            let attempt_result = self
                .forward_once(
                    state.clone(),
                    method.clone(),
                    headers.clone(),
                    body_json,
                    path.clone(),
                    query.clone(),
                )
                .await;

            match attempt_result {
                Ok(response) => {
                    let status_code = i32::from(response.status().as_u16());
                    if response.status().is_success() {
                        log::info!(
                            "[Proxy] app_type={} provider={} success status={} elapsed_ms={}",
                            app_type,
                            state.provider_id,
                            status_code,
                            started_at.elapsed().as_millis()
                        );
                        log::info!(
                            "[Proxy] request_result app_type={} provider={} path={} model={:?} status={} success=true",
                            app_type,
                            state.provider_id,
                            path,
                            request_model,
                            status_code
                        );
                        router
                            .write()
                            .await
                            .record_success(app_type, &state.provider_id);
                        if state.provider_id != current_provider_id {
                            if let Err(e) = failover_switch
                                .try_switch(app_type, &state.provider_id)
                                .await
                            {
                                log::error!("[Proxy] Failed to apply failover switch: {}", e);
                            }
                        }
                        return Ok(ForwardResult {
                            response,
                            provider_id: state.provider_id.clone(),
                        });
                    }
                    router
                        .write()
                        .await
                        .record_failure(app_type, &state.provider_id);
                    log::warn!(
                        "[Proxy] app_type={} provider={} upstream returned non-success status={} (will try next candidate if any)",
                        app_type,
                        state.provider_id,
                        status_code
                    );
                    log::warn!(
                        "[Proxy] request_result app_type={} provider={} path={} model={:?} status={} success=false reason=upstream_status",
                        app_type,
                        state.provider_id,
                        path,
                        request_model,
                        status_code
                    );
                    last_response = Some((state.provider_id.clone(), response));
                }
                Err(status) => {
                    router
                        .write()
                        .await
                        .record_failure(app_type, &state.provider_id);
                    log::error!(
                        "[Proxy] app_type={} provider={} forward failed status={} (will try next candidate if any)",
                        app_type,
                        state.provider_id,
                        status
                    );
                    log::error!(
                        "[Proxy] request_result app_type={} provider={} path={} model={:?} status={} success=false reason=forward_error",
                        app_type,
                        state.provider_id,
                        path,
                        request_model,
                        status
                    );
                    last_status = Some(status);
                }
            }
        }

        if let Some((provider_id, response)) = last_response {
            log::warn!(
                "[Proxy] app_type={} returning last non-success upstream response from provider={} status={} elapsed_ms={}",
                app_type,
                provider_id,
                response.status(),
                started_at.elapsed().as_millis()
            );
            return Ok(ForwardResult {
                response,
                provider_id,
            });
        }

        let final_status = last_status.unwrap_or(StatusCode::BAD_GATEWAY);
        log::error!(
            "[Proxy] app_type={} all provider attempts failed final_status={} elapsed_ms={}",
            app_type,
            final_status,
            started_at.elapsed().as_millis()
        );
        Err(final_status)
    }

    async fn forward_once(
        &self,
        state: Arc<ProxyState>,
        method: axum::http::Method,
        headers: axum::http::HeaderMap,
        body_json: Value,
        path: String,
        query: String,
    ) -> Result<Response, StatusCode> {
        // Resolve auth through the adapter; it decides the provider strategy,
        // while the forwarder only injects the resulting headers.
        let auth_info = state
            .adapter
            .get_auth_info(&state.provider, state.account_id.as_deref())
            .await
            .map_err(|e| {
                log::error!("[Proxy] Failed to resolve auth: {}", e);
                StatusCode::UNAUTHORIZED
            })?;

        // Get account ID from adapter
        let account_id = state
            .adapter
            .extract_account_id(&state.provider)
            .or(state.account_id.clone());

        // Get upstream URL from adapter or config
        let upstream_url = state
            .adapter
            .extract_upstream_url(&state.provider)
            .unwrap_or_else(|| self.config.upstream_url.clone());

        // Get prompt cache key from adapter
        let prompt_cache_key = state
            .adapter
            .extract_prompt_cache_key(&state.provider)
            .or_else(|| self.config.prompt_cache_key.clone());

        let requested_stream = body_json.get("stream").and_then(Value::as_bool) != Some(false);
        let session = extract_session_id(&headers, &body_json);
        let cache_key = build_prompt_cache_key(
            prompt_cache_key.as_deref(),
            session.as_ref().map(|result| result.session_id.as_str()),
            &self.config.prompt_cache_key_fallback,
        );
        log::info!(
            "[Proxy] session_source={:?}, prompt_cache_key_hash={} (len={})",
            session.as_ref().map(|session| session.source),
            cache_key_log_id(&cache_key),
            cache_key.chars().count()
        );

        // Transform request using adapter
        let original_body_json = body_json.clone();
        let transform_input = TransformInput {
            body: body_json,
            upstream_url: upstream_url.clone(),
            prompt_cache_key: Some(cache_key),
            requested_stream,
            codex_fast_mode: self.config.codex_fast_mode,
        };
        let transform_output = state
            .adapter
            .transform_request(transform_input)
            .map_err(|e| {
                log::error!("[Proxy] Failed to transform request for {}: {}", path, e);
                StatusCode::BAD_REQUEST
            })?;

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
            if transform_output
                .headers
                .iter()
                .any(|(k, _)| k == "x-proxy-uri")
            {
                " via configured proxy"
            } else {
                ""
            }
        );

        let reqwest_method =
            Method::from_bytes(method.as_str().as_bytes()).map_err(|_| StatusCode::BAD_REQUEST)?;
        let request_body = serde_json::to_vec(&transform_output.body).map_err(|e| {
            log::error!(
                "[Proxy] Failed to serialize request body for {}: {}",
                path,
                e
            );
            StatusCode::BAD_REQUEST
        })?;

        let mut upstream_req = self
            .http_client
            .request(reqwest_method.clone(), &transform_output.upstream_url)
            .body(request_body);

        let mut auth_headers = state.adapter.get_auth_headers(&auth_info).map_err(|e| {
            log::error!(
                "[Proxy] Failed to build auth headers for {}: {}",
                state.adapter.provider_type(),
                e
            );
            StatusCode::BAD_GATEWAY
        })?;
        if auth_info.strategy == AuthStrategy::CodexOAuth {
            if let Some(account_id) = account_id.as_deref() {
                let value = header::HeaderValue::from_str(account_id).map_err(|e| {
                    log::error!("[Proxy] Invalid Codex account header value: {}", e);
                    StatusCode::BAD_GATEWAY
                })?;
                auth_headers.push((header::HeaderName::from_static("chatgpt-account-id"), value));
            }
        }
        for (name, value) in auth_headers {
            upstream_req = upstream_req.header(name, value);
        }

        // Set headers from adapter
        for (key, value) in &transform_output.headers {
            upstream_req = upstream_req.header(
                header::HeaderName::from_bytes(key.as_bytes())
                    .unwrap_or_else(|_| header::HeaderName::from_static("x-custom-header")),
                value.as_str(),
            );
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

            if should_retry_deepseek_with_rectified_request(
                state.adapter.provider_type(),
                status,
                &excerpt,
            ) {
                log::warn!(
                    "[Proxy] Retrying DeepSeek request with strict rectification for {}",
                    path
                );
                let retry_body = deepseek_strict_rectify_request(original_body_json);
                let retry_response = self
                    .forward_once_with_body(
                        state.clone(),
                        reqwest_method,
                        headers,
                        retry_body,
                        path.clone(),
                        query.clone(),
                    )
                    .await?;
                return Ok(retry_response);
            }

            return Ok(response
                .body(Body::from(body))
                .expect("failed to build error response"));
        }

        if requested_stream {
            let stream = match state.adapter.streaming_response_format() {
                StreamingResponseFormat::Anthropic => {
                    return Ok(response
                        .header(header::CONTENT_TYPE, "text/event-stream")
                        .body(Body::from_stream(upstream_res.bytes_stream()))
                        .unwrap());
                }
                StreamingResponseFormat::OpenAIChat => {
                    let db = state.db.clone();
                    let provider_id = state.provider_id.clone();
                    openai_chat_sse_to_anthropic_with_usage(
                        upstream_res.bytes_stream(),
                        move |usage| {
                            let record = cc_switch_lib::database::UsageRecord {
                                provider_id: provider_id.clone(),
                                model: usage.model,
                                input_tokens: usage.input_tokens,
                                output_tokens: usage.output_tokens,
                                cache_read_tokens: None,
                                request_timestamp: unix_timestamp(),
                            };
                            let db = db.clone();
                            tokio::spawn(async move {
                                if let Err(e) = db.save_usage_record(&record) {
                                    log::error!("[Proxy] Failed to save streaming usage record: {}", e);
                                } else {
                                    log::info!(
                                        "[Proxy] Streaming usage recorded: provider={}, model={}, input={}, output={}",
                                        record.provider_id,
                                        record.model,
                                        record.input_tokens,
                                        record.output_tokens
                                    );
                                }
                            });
                        },
                    )
                    .boxed()
                }
                StreamingResponseFormat::OpenAIResponses => {
                    responses_sse_to_anthropic(upstream_res.bytes_stream()).boxed()
                }
            };
            return Ok(response
                .header(header::CONTENT_TYPE, "text/event-stream")
                .body(Body::from_stream(stream))
                .unwrap());
        }

        let body = upstream_res.bytes().await.map_err(|e| {
            log::error!("[Proxy] Failed to read upstream response body: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

        // Transform response and extract usage using adapter
        let transform_result = state
            .adapter
            .transform_response(body.clone(), false)
            .map_err(|e| {
                log::error!("[Proxy] Failed to transform response: {}", e);
                StatusCode::BAD_GATEWAY
            })?;

        // Record usage if available
        if let Some(mut record) = transform_result.record {
            record.provider_id = state.provider_id.clone();
            let db = state.db.clone();
            tokio::spawn(async move {
                if let Err(e) = db.save_usage_record(&record) {
                    log::error!("[Proxy] Failed to save usage record: {}", e);
                } else {
                    log::info!(
                        "[Proxy] Usage recorded: provider={}, model={}, input={}, output={}",
                        record.provider_id,
                        record.model,
                        record.input_tokens,
                        record.output_tokens
                    );
                }
            });
        }

        Ok(response
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(transform_result.body.to_vec()))
            .unwrap())
    }

    async fn forward_once_with_body(
        &self,
        state: Arc<ProxyState>,
        reqwest_method: Method,
        headers: axum::http::HeaderMap,
        body_json: Value,
        _path: String,
        _query: String,
    ) -> Result<Response, StatusCode> {
        let auth_info = state
            .adapter
            .get_auth_info(&state.provider, state.account_id.as_deref())
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        let upstream_url = state
            .adapter
            .extract_upstream_url(&state.provider)
            .unwrap_or_else(|| self.config.upstream_url.clone());
        let transform_output = state
            .adapter
            .transform_request(TransformInput {
                body: body_json,
                upstream_url,
                prompt_cache_key: None,
                requested_stream: false,
                codex_fast_mode: self.config.codex_fast_mode,
            })
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let request_body = serde_json::to_vec(&transform_output.body).map_err(|_| StatusCode::BAD_REQUEST)?;
        let mut upstream_req = self
            .http_client
            .request(reqwest_method, &transform_output.upstream_url)
            .body(request_body);
        let auth_headers = state
            .adapter
            .get_auth_headers(&auth_info)
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        for (name, value) in auth_headers {
            upstream_req = upstream_req.header(name, value);
        }
        upstream_req = copy_forward_headers(upstream_req, &headers);
        upstream_req = upstream_req.header(header::CONTENT_TYPE, "application/json");
        let upstream_res = upstream_req.send().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
        let status = StatusCode::from_u16(upstream_res.status().as_u16()).unwrap_or(StatusCode::OK);
        let mut response = Response::builder().status(status);
        response = copy_response_headers(response, upstream_res.headers());
        let body = upstream_res.bytes().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
        Ok(response.body(Body::from(body)).expect("failed to build retry response"))
    }
}

fn should_retry_deepseek_with_rectified_request(
    provider_type: &str,
    status: StatusCode,
    error_body: &str,
) -> bool {
    if provider_type != "deepseek" || status != StatusCode::BAD_REQUEST {
        return false;
    }
    let msg = error_body.to_lowercase();
    (msg.contains("thinking") && msg.contains("signature"))
        || (msg.contains("thinking") && msg.contains("budget_tokens"))
}

fn deepseek_strict_rectify_request(mut value: Value) -> Value {
    fn walk(v: &mut Value) {
        match v {
            Value::Object(map) => {
                map.remove("thinking");
                map.remove("signature");
                for child in map.values_mut() {
                    walk(child);
                }
            }
            Value::Array(arr) => {
                arr.retain(|item| {
                    !matches!(
                        item.get("type").and_then(Value::as_str),
                        Some("thinking" | "redacted_thinking")
                    )
                });
                for item in arr.iter_mut() {
                    walk(item);
                }
            }
            _ => {}
        }
    }
    walk(&mut value);
    value
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

fn extract_request_model(body: &Value) -> Option<String> {
    body.get("model")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn cache_key_log_id(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

/// Shared state for proxy server
#[derive(Clone)]
pub struct ProxyState {
    pub adapter: Arc<dyn ProviderAdapter>,
    pub account_id: Option<String>,
    pub provider: cc_switch_lib::database::Provider,
    pub db: Arc<cc_switch_lib::database::Database>,
    pub provider_id: String,
}

impl ProxyState {
    pub fn new(
        adapter: Arc<dyn ProviderAdapter>,
        account_id: Option<String>,
        provider: cc_switch_lib::database::Provider,
        db: Arc<cc_switch_lib::database::Database>,
        provider_id: String,
    ) -> Self {
        Self {
            adapter,
            account_id,
            provider,
            db,
            provider_id,
        }
    }
}
