//! Copilot adapter
//!
//! For GitHub Copilot API with OAuth authentication.

mod response;

use bytes::Bytes;
use cc_switch_lib::database::Provider;
use cc_switch_lib::oauth::CopilotAuthManager;
use cc_switch_lib::providers::{
    AuthInfo, AuthStrategy, BoxFuture, ProviderAdapter, ProviderError, TransformInput,
    TransformOutput, UsageParseResult,
};
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

    fn get_auth_info(
        &self,
        provider: &Provider,
        account_id: Option<&str>,
    ) -> BoxFuture<'_, Result<AuthInfo, ProviderError>> {
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
            let account_id = effective_account_id
                .ok_or_else(|| ProviderError::AuthFailed("No Copilot account ID found".into()))?;

            let token = copilot_auth
                .get_valid_token_for_account(&account_id)
                .await
                .map_err(|e| ProviderError::AuthFailed(e.to_string()))?;

            Ok(AuthInfo::new(token, AuthStrategy::GitHubCopilot))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn transform_request_passthrough_for_copilot() {
        let temp = std::env::temp_dir().join("cc-switch-copilot-test");
        let manager = Arc::new(CopilotAuthManager::new(temp));
        let adapter = CopilotAdapter::new(manager);
        let output = adapter
            .transform_request(TransformInput {
                body: json!({"model":"gpt-4o-mini","messages":[{"role":"user","content":"hi"}]}),
                upstream_url: "https://api.githubcopilot.com/chat/completions".to_string(),
                path: "".to_string(),
                prompt_cache_key: None,
                requested_stream: true,
                codex_fast_mode: false,
            })
            .expect("transform should succeed");

        assert_eq!(output.method, "POST");
        assert_eq!(
            output.upstream_url,
            "https://api.githubcopilot.com/chat/completions"
        );
        assert_eq!(output.body["model"], "gpt-4o-mini");
    }
}
