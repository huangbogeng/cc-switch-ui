//! Claude Auth adapter
//!
//! For relay services that accept Anthropic Messages API with Bearer token auth.

mod response;

use bytes::Bytes;
use cc_switch_lib::database::Provider;
use cc_switch_lib::providers::{
    resolve_provider_api_key, AuthInfo, AuthStrategy, BoxFuture, ProviderAdapter, ProviderError,
    TransformInput, TransformOutput, UsageParseResult,
};

/// Adapter for Claude Auth relay (Bearer token auth, Anthropic format)
pub struct ClaudeAuthAdapter;

impl ClaudeAuthAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClaudeAuthAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderAdapter for ClaudeAuthAdapter {
    fn provider_type(&self) -> &'static str {
        "claude_auth"
    }

    fn get_auth_info(
        &self,
        provider: &Provider,
        _account_id: Option<&str>,
    ) -> BoxFuture<'_, Result<AuthInfo, ProviderError>> {
        let token_result = resolve_provider_api_key(provider);

        Box::pin(async move {
            let token = token_result.ok_or_else(|| {
                ProviderError::AuthFailed("No token found in provider config".into())
            })?;
            Ok(AuthInfo::new(token, AuthStrategy::ClaudeAuth))
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
