//! Claude adapter
//!
//! For providers that accept Anthropic Messages API directly (passthrough).
//! Examples: Claude API (direct), Claude Auth (relay services).

mod response;

use bytes::Bytes;
use cc_switch_lib::database::Provider;
use cc_switch_lib::providers::{
    provider_allows_empty_api_key, resolve_provider_api_key, AuthInfo, AuthStrategy, BoxFuture,
    ProviderAdapter, ProviderError, TransformInput, TransformOutput, UsageParseResult,
};

/// Adapter for Claude API (x-api-key auth, Anthropic format)
pub struct ClaudeAdapter;

impl ClaudeAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderAdapter for ClaudeAdapter {
    fn provider_type(&self) -> &'static str {
        "claude"
    }

    fn get_auth_info(
        &self,
        provider: &Provider,
        _account_id: Option<&str>,
    ) -> BoxFuture<'_, Result<AuthInfo, ProviderError>> {
        let token_result = resolve_provider_api_key(provider);
        let allow_empty_auth = provider_allows_empty_api_key(provider);

        Box::pin(async move {
            match token_result {
                Some(token) => Ok(AuthInfo::new(token, AuthStrategy::Anthropic)),
                None if allow_empty_auth => Ok(AuthInfo::new(String::new(), AuthStrategy::None)),
                None => Err(ProviderError::AuthFailed(
                    "No API key found in provider config".into(),
                )),
            }
        })
    }

    fn transform_request(&self, input: TransformInput) -> Result<TransformOutput, ProviderError> {
        let base = input.upstream_url.trim_end_matches('/');
        let path = input.path.as_str();
        let upstream_url = if path.is_empty() || base.ends_with(path) {
            base.to_string()
        } else {
            format!("{base}{path}")
        };

        // Anthropic payloads are passed through unchanged; only the incoming
        // request path is resolved against a provider's base URL.
        Ok(TransformOutput {
            body: input.body,
            upstream_url,
            headers: vec![],
            method: "POST".to_string(),
        })
    }

    fn transform_response(
        &self,
        body: Bytes,
        is_streaming: bool,
    ) -> Result<UsageParseResult, ProviderError> {
        response::transform(body, is_streaming)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn transform_request_passthrough_for_claude() {
        let adapter = ClaudeAdapter::new();
        let output = adapter
            .transform_request(TransformInput {
                body: json!({"model":"claude-3","messages":[{"role":"user","content":"hi"}]}),
                upstream_url: "https://api.anthropic.com/v1/messages".to_string(),
                path: "".to_string(),
                prompt_cache_key: None,
                requested_stream: true,
                codex_fast_mode: false,
            })
            .expect("transform should succeed");

        assert_eq!(output.method, "POST");
        assert_eq!(output.upstream_url, "https://api.anthropic.com/v1/messages");
        assert_eq!(output.body["model"], "claude-3");
    }

    #[test]
    fn appends_messages_path_to_bare_anthropic_base_url() {
        let adapter = ClaudeAdapter::new();
        let output = adapter
            .transform_request(TransformInput {
                body: json!({"model":"anthropic/claude-sonnet-4.6"}),
                upstream_url: "https://api.orcarouter.ai".to_string(),
                path: "/v1/messages".to_string(),
                prompt_cache_key: None,
                requested_stream: true,
                codex_fast_mode: false,
            })
            .expect("transform should succeed");

        assert_eq!(output.upstream_url, "https://api.orcarouter.ai/v1/messages");
    }

    #[test]
    fn does_not_duplicate_messages_path_for_full_endpoint() {
        let adapter = ClaudeAdapter::new();
        let output = adapter
            .transform_request(TransformInput {
                body: json!({}),
                upstream_url: "https://api.orcarouter.ai/v1/messages".to_string(),
                path: "/v1/messages".to_string(),
                prompt_cache_key: None,
                requested_stream: true,
                codex_fast_mode: false,
            })
            .expect("transform should succeed");

        assert_eq!(output.upstream_url, "https://api.orcarouter.ai/v1/messages");
    }
}
