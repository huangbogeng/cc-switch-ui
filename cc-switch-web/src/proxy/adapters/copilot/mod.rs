//! Copilot adapter
//!
//! For GitHub Copilot API with OAuth authentication.

mod response;

use cc_switch_lib::database::Provider;
use cc_switch_lib::oauth::CopilotAuthManager;
use cc_switch_lib::providers::{
    AuthToken, BoxFuture, ProviderAdapter, ProviderError, TransformInput,
    TransformOutput, UsageParseResult,
};
use bytes::Bytes;
use std::sync::Arc;

/// Adapter for Copilot API (OAuth auth, Copilot format)
pub struct CopilotAdapter {
    copilot_auth: Arc<CopilotAuthManager>,
}

impl CopilotAdapter {
    pub fn new(copilot_auth: Arc<CopilotAuthManager>) -> Self {
        Self { copilot_auth }
    }
}

impl ProviderAdapter for CopilotAdapter {
    fn provider_type(&self) -> &'static str {
        "copilot"
    }

    fn matches_provider(&self, provider: &Provider) -> bool {
        provider
            .meta
            .get("providerType")
            .and_then(|v| v.as_str())
            .map(|t| t.contains("copilot"))
            .unwrap_or(false)
            || provider.id.contains("copilot")
    }

    fn get_auth_token(
        &self,
        provider: &Provider,
        account_id: Option<&str>,
    ) -> BoxFuture<'_, Result<AuthToken, ProviderError>> {
        let copilot_auth = self.copilot_auth.clone();
        let provider_account_id = provider
            .meta
            .get("authBinding")
            .and_then(|v| v.get("accountId"))
            .and_then(|v| v.as_str())
            .map(str::to_string);

        // Use provider's account_id if available, otherwise use the passed account_id
        let effective_account_id = provider_account_id.or_else(|| account_id.map(str::to_string));

        Box::pin(async move {
            let account_id = effective_account_id.ok_or_else(|| {
                ProviderError::AuthFailed("No Copilot account ID found".into())
            })?;

            let token = copilot_auth
                .get_valid_token_for_account(&account_id)
                .await
                .map_err(|e| ProviderError::AuthFailed(e.to_string()))?;

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
