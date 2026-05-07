//! ProviderAdapter trait definition

use super::error::ProviderError;
use super::types::{AuthInfo, AuthStrategy, TransformInput, TransformOutput, UsageParseResult};
use crate::database::Provider;
use bytes::Bytes;
use http::{HeaderName, HeaderValue};

/// Future type for async operations
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// ProviderAdapter trait - all methods are optional with default implementations
pub trait ProviderAdapter: Send + Sync {
    /// Provider type identifier (e.g., "codex_oauth", "copilot", "claude", "openrouter", "gemini")
    fn provider_type(&self) -> &'static str;

    /// Check if this adapter handles the given provider
    fn matches_provider(&self, provider: &Provider) -> bool {
        provider.meta.get("providerType").and_then(|v| v.as_str()) == Some(self.provider_type())
    }

    /// Resolve authentication information for the provider.
    fn get_auth_info(
        &self,
        provider: &Provider,
        account_id: Option<&str>,
    ) -> BoxFuture<'_, Result<AuthInfo, ProviderError>>;

    /// Build all authentication headers required by the upstream provider.
    fn get_auth_headers(
        &self,
        auth: &AuthInfo,
    ) -> Result<Vec<(HeaderName, HeaderValue)>, ProviderError> {
        let bearer = format!("Bearer {}", auth.secret);
        match auth.strategy {
            AuthStrategy::Anthropic => Ok(vec![header("x-api-key", &auth.secret)?]),
            AuthStrategy::ClaudeAuth | AuthStrategy::Bearer => {
                Ok(vec![header("authorization", &bearer)?])
            }
            AuthStrategy::GoogleApiKey => Ok(vec![header("x-goog-api-key", &auth.secret)?]),
            AuthStrategy::GoogleOAuth => {
                let token = auth.access_token.as_ref().unwrap_or(&auth.secret);
                Ok(vec![
                    header("authorization", &format!("Bearer {token}"))?,
                    header("x-goog-api-client", "GeminiCLI/1.0")?,
                ])
            }
            AuthStrategy::GitHubCopilot => Ok(vec![
                header("authorization", &bearer)?,
                header("editor-version", crate::oauth::COPILOT_EDITOR_VERSION)?,
                header(
                    "editor-plugin-version",
                    crate::oauth::COPILOT_PLUGIN_VERSION,
                )?,
                header(
                    "copilot-integration-id",
                    crate::oauth::COPILOT_INTEGRATION_ID,
                )?,
                header("user-agent", crate::oauth::COPILOT_USER_AGENT)?,
                header("x-github-api-version", crate::oauth::COPILOT_API_VERSION)?,
                header("openai-intent", "conversation-agent")?,
                header("x-initiator", "user")?,
                header("x-interaction-type", "conversation-agent")?,
                header("x-vscode-user-agent-library-version", "electron-fetch")?,
            ]),
            AuthStrategy::CodexOAuth => Ok(vec![
                header("authorization", &bearer)?,
                header("originator", "cc-switch")?,
            ]),
        }
    }

    /// Transform outgoing request (optional - passthrough by default)
    fn transform_request(&self, input: TransformInput) -> Result<TransformOutput, ProviderError> {
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

fn header(name: &'static str, value: &str) -> Result<(HeaderName, HeaderValue), ProviderError> {
    let value = HeaderValue::from_str(value).map_err(|e| {
        ProviderError::InvalidConfig(format!("Invalid auth header value for {name}: {e}"))
    })?;
    Ok((HeaderName::from_static(name), value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::UsageParseResult;

    struct TestAdapter;

    impl ProviderAdapter for TestAdapter {
        fn provider_type(&self) -> &'static str {
            "test"
        }

        fn get_auth_info(
            &self,
            _provider: &Provider,
            _account_id: Option<&str>,
        ) -> BoxFuture<'_, Result<AuthInfo, ProviderError>> {
            Box::pin(async { unreachable!("not needed for auth header tests") })
        }

        fn transform_response(
            &self,
            body: Bytes,
            _is_streaming: bool,
        ) -> Result<UsageParseResult, ProviderError> {
            Ok(UsageParseResult { record: None, body })
        }
    }

    fn header_pairs(auth: AuthInfo) -> Vec<(String, String)> {
        TestAdapter
            .get_auth_headers(&auth)
            .unwrap()
            .into_iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    value.to_str().unwrap().to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn anthropic_auth_uses_x_api_key() {
        let headers = header_pairs(AuthInfo::new("sk-ant".to_string(), AuthStrategy::Anthropic));
        assert_eq!(
            headers,
            vec![("x-api-key".to_string(), "sk-ant".to_string())]
        );
    }

    #[test]
    fn bearer_auth_uses_authorization_header() {
        let headers = header_pairs(AuthInfo::new("sk-api".to_string(), AuthStrategy::Bearer));
        assert_eq!(
            headers,
            vec![("authorization".to_string(), "Bearer sk-api".to_string())]
        );
    }

    #[test]
    fn google_api_key_uses_x_goog_api_key() {
        let headers = header_pairs(AuthInfo::new(
            "AIza-test".to_string(),
            AuthStrategy::GoogleApiKey,
        ));
        assert_eq!(
            headers,
            vec![("x-goog-api-key".to_string(), "AIza-test".to_string())]
        );
    }

    #[test]
    fn codex_oauth_adds_originator() {
        let headers = header_pairs(AuthInfo::new(
            "oauth-token".to_string(),
            AuthStrategy::CodexOAuth,
        ));
        assert_eq!(
            headers,
            vec![
                (
                    "authorization".to_string(),
                    "Bearer oauth-token".to_string()
                ),
                ("originator".to_string(), "cc-switch".to_string()),
            ]
        );
    }

    #[test]
    fn copilot_auth_adds_fingerprint_headers() {
        let headers = header_pairs(AuthInfo::new(
            "copilot-token".to_string(),
            AuthStrategy::GitHubCopilot,
        ));
        assert!(headers.contains(&(
            "authorization".to_string(),
            "Bearer copilot-token".to_string()
        )));
        assert!(headers.iter().any(|(name, _)| name == "editor-version"));
        assert!(headers
            .iter()
            .any(|(name, _)| name == "copilot-integration-id"));
    }
}
