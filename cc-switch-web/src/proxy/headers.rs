//! Header handling for proxy forwarding.

use axum::http::{HeaderMap, HeaderName};

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
        "copilot-integration-id",
        "editor-plugin-version",
        "editor-version",
        "host",
        "keep-alive",
        "openai-intent",
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
        "x-goog-api-client",
        "x-goog-api-key",
        "x-github-api-version",
        "x-initiator",
        "x-interaction-type",
        "x-api-key",
        "x-codex-window-id",
        "x-vscode-user-agent-library-version",
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
