//! Generic OpenAI Responses adapter.
//!
//! For API-key providers that natively expose the OpenAI Responses API.

use bytes::Bytes;
use cc_switch_lib::database::Provider;
use cc_switch_lib::providers::{
    provider_allows_empty_api_key, resolve_provider_api_key, AuthInfo, AuthStrategy, BoxFuture,
    ProviderAdapter, ProviderError, StreamingResponseFormat, TransformInput, TransformOutput,
    UsageParseResult,
};

/// Adapter for generic OpenAI Responses-compatible providers.
pub struct ResponsesAdapter;

impl ResponsesAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ResponsesAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderAdapter for ResponsesAdapter {
    fn provider_type(&self) -> &'static str {
        "openai_responses"
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
                Some(token) => Ok(AuthInfo::new(token, AuthStrategy::Bearer)),
                None if allow_empty_auth => Ok(AuthInfo::new(String::new(), AuthStrategy::None)),
                None => Err(ProviderError::AuthFailed(
                    "No API key found in provider config".into(),
                )),
            }
        })
    }

    fn transform_request(&self, input: TransformInput) -> Result<TransformOutput, ProviderError> {
        let transformed = crate::proxy::transform_responses::anthropic_to_codex_responses(
            input.body,
            input.prompt_cache_key.as_deref(),
            input.codex_fast_mode,
        )
        .map_err(ProviderError::TransformFailed)?;

        Ok(TransformOutput {
            body: transformed,
            upstream_url: responses_url(&input.upstream_url),
            headers: vec![],
            method: "POST".to_string(),
        })
    }

    fn transform_response(
        &self,
        body: Bytes,
        is_streaming: bool,
    ) -> Result<UsageParseResult, ProviderError> {
        crate::proxy::adapters::codex::response::transform(body, is_streaming)
    }

    fn streaming_response_format(&self) -> StreamingResponseFormat {
        StreamingResponseFormat::OpenAIResponses
    }
}

fn responses_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.ends_with("/responses") {
        return trimmed.to_string();
    }
    if trimmed.ends_with("/v1") {
        return format!("{trimmed}/responses");
    }
    format!("{trimmed}/v1/responses")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn appends_responses_suffix_to_v1_base_url() {
        assert_eq!(
            responses_url("http://127.0.0.1:11434/v1"),
            "http://127.0.0.1:11434/v1/responses"
        );
    }

    #[test]
    fn transform_request_builds_responses_body() {
        let adapter = ResponsesAdapter::new();
        let output = adapter
            .transform_request(TransformInput {
                body: json!({
                    "model": "gpt-5",
                    "messages": [{"role":"user","content":"hello"}],
                    "stream": true
                }),
                upstream_url: "https://example.com/v1".to_string(),
                path: "/v1/messages".to_string(),
                prompt_cache_key: Some("k1".to_string()),
                requested_stream: true,
                codex_fast_mode: false,
            })
            .expect("transform should succeed");

        assert_eq!(output.method, "POST");
        assert_eq!(output.upstream_url, "https://example.com/v1/responses");
        assert_eq!(output.body["model"], "gpt-5");
        assert_eq!(output.body["stream"], true);
    }
}
