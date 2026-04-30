//! Header handling for Codex OAuth proxy forwarding.

use axum::http::{HeaderMap, HeaderName};

pub fn add_codex_session_headers(
    mut request: reqwest::RequestBuilder,
    session_id: &str,
) -> reqwest::RequestBuilder {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return request;
    }
    let window_id = format!("{session_id}:0");
    request = request
        .header("session_id", session_id)
        .header("x-client-request-id", session_id)
        .header("x-codex-window-id", window_id);
    request
}

pub fn copy_forward_headers(
    mut request: reqwest::RequestBuilder,
    headers: &HeaderMap,
) -> reqwest::RequestBuilder {
    const SKIP_HEADERS: &[&str] = &[
        "accept",
        "accept-encoding",
        "authorization",
        "connection",
        "content-length",
        "content-type",
        "host",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailers",
        "transfer-encoding",
        "upgrade",
        "x-forwarded-host",
        "x-forwarded-port",
        "x-forwarded-proto",
        "forwarded",
        "cf-connecting-ip",
        "claude-code-session-id",
        "cf-ray",
        "true-client-ip",
        "session_id",
        "x-claude-code-session-id",
        "x-client-request-id",
        "x-codex-window-id",
        "x-request-id",
        "x-correlation-id",
        "x-session-id",
        "x-trace-id",
        "traceparent",
        "tracestate",
    ];

    for (name, value) in headers {
        let name_lower = name.as_str().to_ascii_lowercase();
        if SKIP_HEADERS.contains(&name_lower.as_str()) {
            continue;
        }
        if let Ok(header_name) = HeaderName::from_bytes(name.as_str().as_bytes()) {
            request = request.header(header_name, value.clone());
        }
    }
    request
}

pub fn copy_response_headers(
    mut response: axum::http::response::Builder,
    headers: &reqwest::header::HeaderMap,
) -> axum::http::response::Builder {
    const SKIP_HEADERS: &[&str] = &[
        "connection",
        "content-encoding",
        "content-length",
        "content-type",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailers",
        "transfer-encoding",
        "upgrade",
    ];

    for (name, value) in headers {
        let name_lower = name.as_str().to_ascii_lowercase();
        if !SKIP_HEADERS.contains(&name_lower.as_str()) {
            response = response.header(name, value);
        }
    }
    response
}
