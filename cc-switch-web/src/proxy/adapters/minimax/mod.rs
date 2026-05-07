//! MiniMax adapter
//!
//! For MiniMax API with Bearer token auth and OpenAI-compatible format.

mod response;

use bytes::Bytes;
use cc_switch_lib::database::Provider;
use cc_switch_lib::providers::{
    AuthInfo, AuthStrategy, BoxFuture, ProviderAdapter, ProviderError, TransformInput,
    TransformOutput, UsageParseResult,
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
        // Look for token in: env.ANTHROPIC_AUTH_TOKEN, settingsConfig.apiKey, authToken, token
        let token_result = provider
            .settings_config
            .get("env")
            .and_then(|v| v.get("ANTHROPIC_AUTH_TOKEN"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| {
                provider
                    .settings_config
                    .get("apiKey")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .or_else(|| {
                provider
                    .settings_config
                    .get("authToken")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .or_else(|| {
                provider
                    .settings_config
                    .get("token")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            });

        Box::pin(async move {
            let token = token_result.ok_or_else(|| {
                ProviderError::AuthFailed("No API key found in provider config".into())
            })?;
            Ok(AuthInfo::new(token, AuthStrategy::Anthropic))
        })
    }

    fn transform_request(&self, input: TransformInput) -> Result<TransformOutput, ProviderError> {
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
