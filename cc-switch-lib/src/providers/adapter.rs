//! ProviderAdapter trait definition

use super::error::ProviderError;
use super::types::{AuthToken, TransformInput, TransformOutput, UsageParseResult};
use crate::database::Provider;
use bytes::Bytes;

/// Future type for async operations
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// ProviderAdapter trait - all methods are optional with default implementations
pub trait ProviderAdapter: Send + Sync {
    /// Provider type identifier (e.g., "codex_oauth", "copilot", "claude", "openrouter", "gemini")
    fn provider_type(&self) -> &'static str;

    /// Check if this adapter handles the given provider
    fn matches_provider(&self, provider: &Provider) -> bool {
        provider
            .meta
            .get("providerType")
            .and_then(|v| v.as_str())
            == Some(self.provider_type())
    }

    /// Get auth token for the provider (required)
    fn get_auth_token(
        &self,
        provider: &Provider,
        account_id: Option<&str>,
    ) -> BoxFuture<'_, Result<AuthToken, ProviderError>>;

    /// Transform outgoing request (optional - passthrough by default)
    fn transform_request(
        &self,
        input: TransformInput,
    ) -> Result<TransformOutput, ProviderError> {
        Ok(TransformOutput {
            body: input.body,
            upstream_url: input.upstream_url,
            headers: vec![],
            method: "POST".to_string(),
        })
    }

    /// Transform response and extract usage
    /// Returns the transformed body and usage record if available
    fn transform_response(
        &self,
        body: Bytes,
        is_streaming: bool,
    ) -> Result<UsageParseResult, ProviderError>;

    /// Get account ID from provider metadata
    fn extract_account_id(&self, provider: &Provider) -> Option<String> {
        provider
            .meta
            .get("authBinding")
            .and_then(|v| v.as_str())
            .map(str::to_string)
    }

    /// Get HTTP proxy URL from provider metadata
    fn extract_http_proxy(&self, provider: &Provider) -> Option<String> {
        provider
            .meta
            .get("codexHttpProxy")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
    }

    /// Get prompt cache key from provider metadata
    fn extract_prompt_cache_key(&self, provider: &Provider) -> Option<String> {
        provider
            .meta
            .get("promptCacheKey")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
    }

    /// Get upstream URL from provider settings
    fn extract_upstream_url(&self, provider: &Provider) -> Option<String> {
        provider
            .settings_config
            .get("baseUrl")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
    }
}
