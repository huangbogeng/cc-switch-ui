//! Session and cache routing helpers for Codex OAuth proxying.

use axum::http::HeaderMap;
use serde_json::Value;

const PROMPT_CACHE_KEY_MAX_CHARS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionIdSource {
    Header,
    MetadataUserId,
    MetadataSessionId,
    PreviousResponseId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionIdResult {
    pub session_id: String,
    pub source: SessionIdSource,
    pub client_provided: bool,
}

pub fn extract_session_id(headers: &HeaderMap, body: &Value) -> Option<SessionIdResult> {
    extract_from_headers(headers)
        .or_else(|| extract_from_metadata_user_id(body))
        .or_else(|| extract_from_metadata_session_id(body))
        .or_else(|| extract_from_previous_response_id(body))
}

pub fn build_prompt_cache_key(
    explicit_cache_key: Option<&str>,
    session_id: Option<&str>,
    fallback: &str,
) -> String {
    let raw = explicit_cache_key
        .and_then(non_empty)
        .or_else(|| session_id.and_then(non_empty))
        .or_else(|| non_empty(fallback))
        .unwrap_or("codex_oauth");
    normalize_prompt_cache_key(raw)
}

pub fn normalize_prompt_cache_key(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.chars().count() <= PROMPT_CACHE_KEY_MAX_CHARS {
        return trimmed.to_string();
    }

    let hash = fnv1a64_hex(trimmed);
    let prefix: String = trimmed
        .chars()
        .take(PROMPT_CACHE_KEY_MAX_CHARS - 17)
        .collect();
    format!("{prefix}-{hash}")
}

fn extract_from_headers(headers: &HeaderMap) -> Option<SessionIdResult> {
    for header_name in [
        "x-claude-code-session-id",
        "claude-code-session-id",
        "session_id",
        "x-session-id",
    ] {
        let session_id = headers
            .get(header_name)
            .and_then(|value| value.to_str().ok())
            .and_then(non_empty);
        if let Some(session_id) = session_id {
            return Some(SessionIdResult {
                session_id: session_id.to_string(),
                source: SessionIdSource::Header,
                client_provided: true,
            });
        }
    }
    None
}

fn extract_from_metadata_user_id(body: &Value) -> Option<SessionIdResult> {
    let session_id = body
        .get("metadata")
        .and_then(|metadata| metadata.get("user_id"))
        .and_then(Value::as_str)
        .and_then(parse_session_from_user_id)?;
    Some(SessionIdResult {
        session_id,
        source: SessionIdSource::MetadataUserId,
        client_provided: true,
    })
}

fn extract_from_metadata_session_id(body: &Value) -> Option<SessionIdResult> {
    let session_id = body
        .get("metadata")
        .and_then(|metadata| metadata.get("session_id"))
        .and_then(Value::as_str)
        .and_then(extract_inner_session_id)?;
    Some(SessionIdResult {
        session_id,
        source: SessionIdSource::MetadataSessionId,
        client_provided: true,
    })
}

fn extract_from_previous_response_id(body: &Value) -> Option<SessionIdResult> {
    let session_id = body
        .get("previous_response_id")
        .and_then(Value::as_str)
        .and_then(non_empty)?;
    Some(SessionIdResult {
        session_id: session_id.to_string(),
        source: SessionIdSource::PreviousResponseId,
        client_provided: true,
    })
}

fn parse_session_from_user_id(user_id: &str) -> Option<String> {
    let marker = "_session_";
    let pos = user_id.find(marker)?;
    non_empty(&user_id[pos + marker.len()..]).map(ToString::to_string)
}

fn extract_inner_session_id(value: &str) -> Option<String> {
    let trimmed = non_empty(value)?;
    if !trimmed.starts_with('{') {
        return Some(trimmed.to_string());
    }
    serde_json::from_str::<Value>(trimmed)
        .ok()
        .and_then(|parsed| {
            parsed
                .get("session_id")
                .and_then(Value::as_str)
                .and_then(non_empty)
                .map(ToString::to_string)
        })
        .or_else(|| Some(trimmed.to_string()))
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn fnv1a64_hex(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use serde_json::json;

    #[test]
    fn extracts_header_before_metadata() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-claude-code-session-id",
            HeaderValue::from_static("header-session"),
        );
        let body = json!({
            "metadata": {
                "user_id": "user_session_body-session"
            }
        });

        let result = extract_session_id(&headers, &body).unwrap();

        assert_eq!(result.session_id, "header-session");
        assert_eq!(result.source, SessionIdSource::Header);
    }

    #[test]
    fn extracts_session_from_metadata_user_id() {
        let body = json!({
            "metadata": {
                "user_id": "user_session_body-session"
            }
        });

        let result = extract_session_id(&HeaderMap::new(), &body).unwrap();

        assert_eq!(result.session_id, "body-session");
        assert_eq!(result.source, SessionIdSource::MetadataUserId);
    }

    #[test]
    fn extracts_session_from_json_encoded_metadata_session_id() {
        let body = json!({
            "metadata": {
                "session_id": r#"{"device_id":"device","account_uuid":"","session_id":"d0a5ff55-19c0-4ec5-83c5-9256a54f3679"}"#
            }
        });

        let result = extract_session_id(&HeaderMap::new(), &body).unwrap();

        assert_eq!(result.session_id, "d0a5ff55-19c0-4ec5-83c5-9256a54f3679");
        assert_eq!(result.source, SessionIdSource::MetadataSessionId);
    }

    #[test]
    fn extracts_plain_metadata_session_id() {
        let body = json!({
            "metadata": {
                "session_id": "plain-session"
            }
        });

        let result = extract_session_id(&HeaderMap::new(), &body).unwrap();

        assert_eq!(result.session_id, "plain-session");
        assert_eq!(result.source, SessionIdSource::MetadataSessionId);
    }

    #[test]
    fn extracts_previous_response_id_as_fallback() {
        let body = json!({ "previous_response_id": "resp_123456789" });

        let result = extract_session_id(&HeaderMap::new(), &body).unwrap();

        assert_eq!(result.session_id, "resp_123456789");
        assert_eq!(result.source, SessionIdSource::PreviousResponseId);
    }

    #[test]
    fn prompt_cache_key_prefers_explicit_then_session_and_caps_length() {
        let explicit = "explicit-".repeat(20);
        let key = build_prompt_cache_key(Some(&explicit), Some("session"), "fallback");

        assert!(key.starts_with("explicit-"));
        assert_eq!(key.chars().count(), PROMPT_CACHE_KEY_MAX_CHARS);

        let session_key = build_prompt_cache_key(None, Some("session"), "fallback");
        assert_eq!(session_key, "session");
    }

    #[test]
    fn prompt_cache_key_uses_fallback_for_empty_sources() {
        let key = build_prompt_cache_key(Some(" "), Some(""), "provider");

        assert_eq!(key, "provider");
    }
}
