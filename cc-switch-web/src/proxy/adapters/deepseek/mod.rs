//! DeepSeek adapter
//!
//! For DeepSeek API with Bearer token auth and OpenAI-compatible format.

mod request;
mod response;

use bytes::Bytes;
use cc_switch_lib::database::Provider;
use cc_switch_lib::providers::{
    AuthInfo, AuthStrategy, BoxFuture, ProviderAdapter, ProviderError, StreamingResponseFormat,
    TransformInput, TransformOutput, UsageParseResult,
};

/// Adapter for DeepSeek API (Bearer token auth, OpenAI format)
pub struct DeepSeekAdapter;

impl DeepSeekAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DeepSeekAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderAdapter for DeepSeekAdapter {
    fn provider_type(&self) -> &'static str {
        "deepseek"
    }

    fn get_auth_info(
        &self,
        provider: &Provider,
        _account_id: Option<&str>,
    ) -> BoxFuture<'_, Result<AuthInfo, ProviderError>> {
        let env = provider.settings_config.get("env");
        let token_result = provider
            .settings_config
            .get("authToken")
            .and_then(|v| v.as_str())
            .or_else(|| {
                provider
                    .settings_config
                    .get("apiKey")
                    .and_then(|v| v.as_str())
            })
            .or_else(|| {
                env.and_then(|v| v.get("ANTHROPIC_AUTH_TOKEN"))
                    .and_then(|v| v.as_str())
            })
            .or_else(|| {
                env.and_then(|v| v.get("ANTHROPIC_API_KEY"))
                    .and_then(|v| v.as_str())
            })
            .map(str::to_string);

        Box::pin(async move {
            let token = token_result.ok_or_else(|| {
                ProviderError::AuthFailed("No API key found in provider config".into())
            })?;
            Ok(AuthInfo::new(token, AuthStrategy::Bearer))
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
        provider
            .settings_config
            .get("baseUrl")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
            .or_else(|| {
                provider
                    .settings_config
                    .get("env")
                    .and_then(|v| v.get("ANTHROPIC_BASE_URL"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string)
            })
    }
}
