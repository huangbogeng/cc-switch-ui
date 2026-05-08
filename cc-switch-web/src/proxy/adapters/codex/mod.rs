//! Codex OAuth adapter
//!
//! Handles Anthropic -> Codex Responses request transformation
//! and extracts usage from OpenAI-format responses.

mod request;
mod response;

use bytes::Bytes;
use cc_switch_lib::database::Provider;
use cc_switch_lib::oauth::codex::CodexOAuthManager;
use cc_switch_lib::providers::{
    AuthInfo, AuthStrategy, BoxFuture, ProviderAdapter, ProviderError, StreamingResponseFormat,
    TransformInput, TransformOutput, UsageParseResult,
};
use std::sync::Arc;

/// Adapter for Codex OAuth provider
pub struct CodexAdapter {
    codex_oauth: Arc<CodexOAuthManager>,
}

impl CodexAdapter {
    pub fn new(codex_oauth: Arc<CodexOAuthManager>) -> Self {
        Self { codex_oauth }
    }
}

impl ProviderAdapter for CodexAdapter {
    fn provider_type(&self) -> &'static str {
        "codex_oauth"
    }

    fn get_auth_info(
        &self,
        provider: &Provider,
        account_id: Option<&str>,
    ) -> BoxFuture<'_, Result<AuthInfo, ProviderError>> {
        let codex_oauth = self.codex_oauth.clone();
        let resolved_account_id = provider
            .meta
            .get("authBinding")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or(account_id.map(str::to_string));

        Box::pin(async move {
            let token = match resolved_account_id.as_deref() {
                Some(id) => codex_oauth.get_valid_token_for_account(id).await,
                None => codex_oauth.get_valid_token().await,
            }
            .map_err(|e| ProviderError::TokenFailed(e.to_string()))?;

            Ok(AuthInfo::new(token, AuthStrategy::CodexOAuth))
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
        StreamingResponseFormat::OpenAIResponses
    }

    fn extract_upstream_url(&self, _provider: &Provider) -> Option<String> {
        Some("https://chatgpt.com/backend-api/codex/responses".to_string())
    }
}
