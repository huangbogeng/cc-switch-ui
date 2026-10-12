//! Select the wire protocol independently from provider authentication, as in
//! cc-switch's Claude adapter. No prompt rewriting or Copilot optimizers here.
use super::{ClaudeAdapter, MiniMaxAdapter, ResponsesAdapter};
use bytes::Bytes;
use cc_switch_lib::database::Provider;
use cc_switch_lib::providers::{
    AuthInfo, BoxFuture, ProviderAdapter, ProviderError, StreamingResponseFormat, TransformInput,
    TransformOutput, UsageParseResult,
};
use http::{HeaderName, HeaderValue};
use std::sync::Arc;

pub struct ProtocolAdapter {
    inner: Arc<dyn ProviderAdapter>,
    format: String,
    preserve_reasoning: bool,
}

impl ProtocolAdapter {
    pub fn wrap(inner: Arc<dyn ProviderAdapter>, provider: &Provider) -> Arc<dyn ProviderAdapter> {
        // Managed Codex credentials always use the Codex Responses contract.
        if inner.provider_type() == "codex_oauth" {
            return inner;
        }
        let format = Self::format(inner.as_ref(), provider);
        // Generic Chat authentication falls back to the DeepSeek adapter. That
        // fallback must not enable a vendor-only reasoning_content field.
        let preserve_reasoning = provider.id == "deepseek"
            || provider
                .meta
                .get("providerType")
                .and_then(serde_json::Value::as_str)
                == Some("deepseek")
            || inner
                .extract_upstream_url(provider)
                .and_then(|url| reqwest::Url::parse(&url).ok())
                .and_then(|url| url.host_str().map(str::to_string))
                .is_some_and(|host| host == "api.deepseek.com" || host.ends_with(".deepseek.com"));
        Arc::new(Self {
            inner,
            format,
            preserve_reasoning,
        })
    }

    pub fn validate(inner: &dyn ProviderAdapter, provider: &Provider) -> Result<(), String> {
        if inner.provider_type() == "codex_oauth" {
            return Ok(());
        }
        match Self::format(inner, provider).as_str() {
            "anthropic" | "openai_chat" | "openai_responses" => Ok(()),
            "gemini_native" => Err("Gemini native conversion is not implemented; choose an Anthropic-compatible endpoint for local routing".into()),
            other => Err(format!("Unsupported API format: {other}")),
        }
    }

    fn format(inner: &dyn ProviderAdapter, provider: &Provider) -> String {
        let default = match inner.provider_type() {
            "deepseek" | "minimax" | "openrouter" | "siliconflow" | "copilot" => "openai_chat",
            "openai_responses" => "openai_responses",
            "gemini" => "gemini_native",
            _ => "anthropic",
        };
        provider
            .meta
            .get("apiFormat")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                provider
                    .settings_config
                    .get("api_format")
                    .and_then(serde_json::Value::as_str)
            })
            .unwrap_or(default)
            .to_string()
    }
}

impl ProviderAdapter for ProtocolAdapter {
    fn provider_type(&self) -> &'static str {
        self.inner.provider_type()
    }
    fn get_auth_info(
        &self,
        provider: &Provider,
        account_id: Option<&str>,
    ) -> BoxFuture<'_, Result<AuthInfo, ProviderError>> {
        self.inner.get_auth_info(provider, account_id)
    }
    fn get_auth_headers(
        &self,
        auth: &AuthInfo,
    ) -> Result<Vec<(HeaderName, HeaderValue)>, ProviderError> {
        self.inner.get_auth_headers(auth)
    }
    fn extract_account_id(&self, provider: &Provider) -> Option<String> {
        self.inner.extract_account_id(provider)
    }
    fn extract_upstream_url(&self, provider: &Provider) -> Option<String> {
        self.inner.extract_upstream_url(provider)
    }
    fn extract_prompt_cache_key(&self, provider: &Provider) -> Option<String> {
        self.inner.extract_prompt_cache_key(provider)
    }

    fn transform_request(&self, input: TransformInput) -> Result<TransformOutput, ProviderError> {
        if input.path.ends_with("/count_tokens") && self.format != "anthropic" {
            return Err(ProviderError::TransformFailed(
                "Token counting is available only for Anthropic-format routes".into(),
            ));
        }
        match self.format.as_str() {
            "anthropic" => ClaudeAdapter::new().transform_request(input),
            "openai_chat" => {
                let mut body = crate::proxy::transform_chat::anthropic_to_openai_chat(
                    input.body, input.requested_stream, self.preserve_reasoning,
                ).map_err(ProviderError::TransformFailed)?;
                // Copilot's Chat endpoint rejects stop sequences.
                if self.inner.provider_type() == "copilot" { body.as_object_mut().unwrap().remove("stop"); }
                Ok(TransformOutput { body, upstream_url:crate::proxy::transform_chat::chat_url(&input.upstream_url), headers:vec![], method:"POST".into() })
            }
            "openai_responses" => ResponsesAdapter::new().transform_request(input),
            "gemini_native" => Err(ProviderError::TransformFailed("Gemini native conversion is not implemented; use an Anthropic-compatible endpoint for this route".into())),
            other => Err(ProviderError::TransformFailed(format!("Unsupported API format: {other}"))),
        }
    }
    fn transform_response(
        &self,
        body: Bytes,
        streaming: bool,
    ) -> Result<UsageParseResult, ProviderError> {
        match self.format.as_str() {
            "anthropic" => ClaudeAdapter::new().transform_response(body, streaming),
            "openai_chat" => MiniMaxAdapter::new().transform_response(body, streaming),
            "openai_responses" => ResponsesAdapter::new().transform_response(body, streaming),
            other => Err(ProviderError::ResponseTransformFailed(format!(
                "Unsupported API format: {other}"
            ))),
        }
    }
    fn streaming_response_format(&self) -> StreamingResponseFormat {
        match self.format.as_str() {
            "openai_chat" => StreamingResponseFormat::OpenAIChat,
            "openai_responses" => StreamingResponseFormat::OpenAIResponses,
            _ => StreamingResponseFormat::Anthropic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn provider(format: &str) -> Provider {
        Provider {
            id: "deepseek".into(),
            name: "test".into(),
            settings_config: json!({"env":{"ANTHROPIC_AUTH_TOKEN":"test"}}),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            icon: None,
            icon_color: None,
            meta: json!({"apiFormat":format}),
            in_failover_queue: false,
        }
    }
    fn input(body: serde_json::Value) -> TransformInput {
        TransformInput {
            body,
            upstream_url: "https://example.com".into(),
            path: "/v1/messages".into(),
            prompt_cache_key: None,
            requested_stream: true,
            codex_fast_mode: false,
        }
    }
    #[test]
    fn explicit_native_format_overrides_provider_identity_without_rewriting_prompts() {
        let adapter = ProtocolAdapter::wrap(
            Arc::new(super::super::DeepSeekAdapter::new()),
            &provider("anthropic"),
        );
        let body = json!({"system":[{"type":"text","text":"x-anthropic-billing-header: unchanged"}],"messages":[{"role":"system","content":"keep"}],"thinking":{"type":"adaptive","display":"omitted"},"context_management":{"edits":[]},"new_field":{"value":1}});
        let output = adapter.transform_request(input(body.clone())).unwrap();
        assert_eq!(output.body, body);
        assert_eq!(output.upstream_url, "https://example.com/v1/messages");
        assert_eq!(
            adapter.streaming_response_format(),
            StreamingResponseFormat::Anthropic
        );
    }
    #[test]
    fn chat_provider_gets_request_and_response_conversion() {
        let adapter = ProtocolAdapter::wrap(
            Arc::new(super::super::OpenRouterAdapter::new()),
            &provider("openai_chat"),
        );
        let output = adapter
            .transform_request(input(
                json!({"system":"keep rules","messages":[{"role":"user","content":"hello"}]}),
            ))
            .unwrap();
        assert_eq!(output.body["messages"][0]["role"], "system");
        assert_eq!(
            output.upstream_url,
            "https://example.com/v1/chat/completions"
        );
        assert_eq!(
            adapter.streaming_response_format(),
            StreamingResponseFormat::OpenAIChat
        );
        let output=adapter.transform_response(Bytes::from_static(br#"{"id":"r","model":"m","choices":[{"message":{"content":"ok"},"finish_reason":"stop"}]}"#),false).unwrap();
        let body: serde_json::Value = serde_json::from_slice(&output.body).unwrap();
        assert_eq!(body["type"], "message");
        assert_eq!(body["content"][0]["text"], "ok");
    }
    #[test]
    fn unsupported_protocol_fails_before_upstream_request() {
        let adapter =
            ProtocolAdapter::wrap(Arc::new(ClaudeAdapter::new()), &provider("gemini_native"));
        assert!(adapter
            .transform_request(input(json!({})))
            .unwrap_err()
            .to_string()
            .contains("not implemented"));
    }

    #[test]
    fn generic_chat_auth_fallback_does_not_enable_vendor_reasoning_fields() {
        let mut provider = provider("openai_chat");
        provider.id = "local-chat".into();
        let adapter =
            ProtocolAdapter::wrap(Arc::new(super::super::DeepSeekAdapter::new()), &provider);
        let output = adapter.transform_request(input(json!({"messages":[{"role":"assistant","content":[{"type":"thinking","thinking":"vendor private reasoning"},{"type":"text","text":"visible"}]}]}))).unwrap();
        assert_eq!(output.body["messages"][0]["content"], "visible");
        assert!(output.body["messages"][0]
            .get("reasoning_content")
            .is_none());
    }
}
