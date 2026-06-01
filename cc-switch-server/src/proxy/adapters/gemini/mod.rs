//! Gemini adapter
//!
//! For Google Gemini API with x-goog-api-key auth and Gemini native format.

mod response;

use bytes::Bytes;
use cc_switch_lib::database::Provider;
use cc_switch_lib::providers::{
    provider_allows_empty_api_key, resolve_provider_api_key, AuthInfo, AuthStrategy, BoxFuture,
    ProviderAdapter, ProviderError, TransformInput, TransformOutput, UsageParseResult,
};

/// Adapter for Gemini API (x-goog-api-key auth, Gemini native format)
pub struct GeminiAdapter;

impl GeminiAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GeminiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderAdapter for GeminiAdapter {
    fn provider_type(&self) -> &'static str {
        "gemini"
    }

    fn matches_provider(&self, provider: &Provider) -> bool {
        // Match by providerType "gemini" or provider ID containing "gemini"
        provider
            .meta
            .get("providerType")
            .and_then(|v| v.as_str())
            .map(|t| t.contains("gemini"))
            .unwrap_or(false)
            || provider.id.contains("gemini")
    }

    fn get_auth_info(
        &self,
        provider: &Provider,
        _account_id: Option<&str>,
    ) -> BoxFuture<'_, Result<AuthInfo, ProviderError>> {
        let token_result = resolve_provider_api_key(provider).or_else(|| {
            provider
                .settings_config
                .get("env")
                .and_then(|v| v.get("GEMINI_API_KEY"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
        });
        let allow_empty_auth = provider_allows_empty_api_key(provider);

        Box::pin(async move {
            match token_result {
                Some(token) => Ok(AuthInfo::new(token, AuthStrategy::GoogleApiKey)),
                None if allow_empty_auth => Ok(AuthInfo::new(String::new(), AuthStrategy::None)),
                None => Err(ProviderError::AuthFailed(
                    "No API key found in provider config".into(),
                )),
            }
        })
    }

    fn transform_request(&self, input: TransformInput) -> Result<TransformOutput, ProviderError> {
        // Passthrough for now - Gemini native format conversion would go here
        Ok(TransformOutput {
            body: input.body,
            upstream_url: input.upstream_url,
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
    fn transform_request_passthrough_for_gemini() {
        let adapter = GeminiAdapter::new();
        let output = adapter
            .transform_request(TransformInput {
                body: json!({"model":"gemini-2.5-pro","messages":[{"role":"user","content":"hi"}]}),
                upstream_url: "https://generativelanguage.googleapis.com/v1beta/models".to_string(),
                path: "".to_string(),
                prompt_cache_key: None,
                requested_stream: false,
                codex_fast_mode: false,
            })
            .expect("transform should succeed");

        assert_eq!(output.method, "POST");
        assert_eq!(
            output.upstream_url,
            "https://generativelanguage.googleapis.com/v1beta/models"
        );
        assert_eq!(output.body["model"], "gemini-2.5-pro");
    }
}
