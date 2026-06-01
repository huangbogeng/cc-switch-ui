//! OpenRouter adapter
//!
//! For OpenRouter API with Bearer token auth and OpenAI-compatible format.
//! OpenRouter uses specific model names like "anthropic/claude-sonnet-4.6".

mod response;

use bytes::Bytes;
use cc_switch_lib::database::Provider;
use cc_switch_lib::providers::{
    provider_allows_empty_api_key, resolve_provider_api_key, AuthInfo, AuthStrategy, BoxFuture,
    ProviderAdapter, ProviderError, TransformInput, TransformOutput, UsageParseResult,
};

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

}
