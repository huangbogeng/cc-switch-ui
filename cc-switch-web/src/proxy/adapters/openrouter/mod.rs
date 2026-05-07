//! OpenRouter adapter
//!
//! For OpenRouter API with Bearer token auth and OpenAI-compatible format.
//! OpenRouter uses specific model names like "anthropic/claude-sonnet-4.6".

mod response;

use cc_switch_lib::database::Provider;
use cc_switch_lib::providers::{
    AuthToken, BoxFuture, ProviderAdapter, ProviderError, TransformInput,
    TransformOutput, UsageParseResult,
};
use bytes::Bytes;

/// Adapter for OpenRouter API (Bearer token auth, OpenAI format)
pub struct OpenRouterAdapter;

impl OpenRouterAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenRouterAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderAdapter for OpenRouterAdapter {
    fn provider_type(&self) -> &'static str {
        "openrouter"
    }

    fn get_auth_token(
        &self,
        provider: &Provider,
        _account_id: Option<&str>,
    ) -> BoxFuture<'_, Result<AuthToken, ProviderError>> {
        let token_result = provider
            .settings_config
            .get("authToken")
            .and_then(|v| v.as_str())
            .or_else(|| provider.settings_config.get("apiKey").and_then(|v| v.as_str()))
            .or_else(|| provider.settings_config.get("api_key").and_then(|v| v.as_str()))
            .map(str::to_string);

        Box::pin(async move {
            let token = token_result.ok_or_else(|| {
                ProviderError::AuthFailed("No API key found in provider config".into())
            })?;
            Ok(AuthToken {
                token,
                expires_at_ms: None,
            })
        })
    }

    fn transform_request(
        &self,
        input: TransformInput,
    ) -> Result<TransformOutput, ProviderError> {
        // Passthrough - no transformation needed
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

    fn extract_upstream_url(&self, provider: &Provider) -> Option<String> {
        provider
            .settings_config
            .get("baseUrl")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
    }
}
