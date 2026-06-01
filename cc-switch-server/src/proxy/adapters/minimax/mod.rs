//! MiniMax adapter
//!
//! For MiniMax API with Bearer token auth and OpenAI-compatible format.

mod request;
mod response;

use bytes::Bytes;
use cc_switch_lib::database::Provider;
use cc_switch_lib::providers::{
    provider_allows_empty_api_key, resolve_provider_api_key, AuthInfo, AuthStrategy, BoxFuture,
    ProviderAdapter, ProviderError, StreamingResponseFormat, TransformInput, TransformOutput,
    UsageParseResult,
};

/// Adapter for MiniMax API (Bearer token auth, OpenAI format)
pub struct MiniMaxAdapter;

impl MiniMaxAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MiniMaxAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderAdapter for MiniMaxAdapter {
    fn provider_type(&self) -> &'static str {
        "minimax"
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
        request::transform(input)
    }

    fn transform_response(
        &self,
        body: Bytes,
        is_streaming: bool,
    ) -> Result<UsageParseResult, ProviderError> {
        response::transform(body, is_streaming)
    }

    fn streaming_response_format(&self) -> StreamingResponseFormat {
        StreamingResponseFormat::OpenAIChat
    }

    fn extract_upstream_url(&self, provider: &Provider) -> Option<String> {
        // Look for base URL in: env.ANTHROPIC_BASE_URL, baseUrl
        provider
            .settings_config
            .get("env")
            .and_then(|v| v.get("ANTHROPIC_BASE_URL"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
            .or_else(|| {
                provider
                    .settings_config
                    .get("baseUrl")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string)
            })
    }
}
