//! Provider registry for looking up adapters

use super::adapter::ProviderAdapter;
use crate::database::Provider;
use std::collections::HashMap;
use std::sync::Arc;

/// Provider registry for looking up adapters by type
pub struct ProviderRegistry {
    adapters: HashMap<&'static str, Arc<dyn ProviderAdapter>>,
}

impl ProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    /// Register an adapter
    pub fn register<A: ProviderAdapter + 'static>(&mut self, adapter: Arc<A>) {
        self.adapters
            .insert(adapter.provider_type(), adapter as Arc<dyn ProviderAdapter>);
    }

    /// Get an adapter by provider type
    pub fn get(&self, provider_type: &str) -> Option<Arc<dyn ProviderAdapter>> {
        self.adapters.get(provider_type).cloned()
    }

    /// Find an adapter for a provider by examining its metadata
    /// First tries meta.providerType, then falls back to provider ID,
    /// then tries partial ID matching (for providers like "gemini-native" -> "gemini"),
    /// and finally falls back to meta.apiFormat for custom providers.
    pub fn find_for_provider(&self, provider: &Provider) -> Option<Arc<dyn ProviderAdapter>> {
        // Try meta.providerType first
        if let Some(provider_type) = provider.meta.get("providerType").and_then(|v| v.as_str()) {
            if let Some(adapter) = self.get(provider_type) {
                return Some(adapter);
            }
            // Try partial match on providerType (e.g., "copilot" in "copilot_oauth")
            for (key, adapter) in &self.adapters {
                if *key != provider_type && provider_type.contains(key) {
                    return Some(adapter.clone());
                }
            }
        }
        // Fall back to provider ID (e.g., "minimax", "openrouter", "deepseek")
        if let Some(adapter) = self.get(&provider.id) {
            return Some(adapter);
        }
        // Try partial match on provider ID (e.g., "gemini-native" -> "gemini")
        for (key, adapter) in &self.adapters {
            if provider.id.contains(key) {
                return Some(adapter.clone());
            }
        }
        // Fallback: use apiFormat to find a suitable adapter for custom providers
        // that don't have providerType set in metadata (e.g., manually-configured
        // providers created without a preset).
        let api_format = provider
            .meta
            .get("apiFormat")
            .and_then(|v| v.as_str())
            .unwrap_or("anthropic");
        match api_format {
            // OpenAI Chat → generic Bearer-token chat adapter
            "openai_chat" => self.get("deepseek"),
            // OpenAI Responses → generic Responses adapter
            "openai_responses" => self.get("openai_responses"),
            "gemini_native" => self.get("gemini"),
            // Default to Claude adapter for "anthropic" and unknown formats
            // (passthrough with x-api-key auth)
            _ => self.get("claude"),
        }
    }

    /// Check if a provider type is registered
    pub fn contains(&self, provider_type: &str) -> bool {
        self.adapters.contains_key(provider_type)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Provider;
    use crate::providers::{
        AuthInfo, AuthStrategy, BoxFuture, ProviderAdapter, TransformInput, TransformOutput,
        UsageParseResult,
    };
    use bytes::Bytes;
    use serde_json::json;

    struct TestAdapter(&'static str);

    impl ProviderAdapter for TestAdapter {
        fn provider_type(&self) -> &'static str {
            self.0
        }

        fn get_auth_info(
            &self,
            _provider: &Provider,
            _account_id: Option<&str>,
        ) -> BoxFuture<'_, Result<AuthInfo, crate::providers::ProviderError>> {
            Box::pin(async { Ok(AuthInfo::new(String::new(), AuthStrategy::None)) })
        }

        fn transform_request(
            &self,
            input: TransformInput,
        ) -> Result<TransformOutput, crate::providers::ProviderError> {
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
            _is_streaming: bool,
        ) -> Result<UsageParseResult, crate::providers::ProviderError> {
            Ok(UsageParseResult { record: None, body })
        }
    }

    fn provider(id: &str, meta: serde_json::Value) -> Provider {
        Provider {
            id: id.to_string(),
            name: id.to_string(),
            settings_config: json!({ "env": { "ANTHROPIC_BASE_URL": "http://127.0.0.1:11434/v1" } }),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            icon: None,
            icon_color: None,
            meta,
            in_failover_queue: false,
        }
    }

    #[test]
    fn openai_responses_falls_back_to_responses_adapter() {
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(TestAdapter("openai_responses")));

        let provider = provider(
            "local-responses",
            json!({ "apiFormat": "openai_responses" }),
        );
        let adapter = registry
            .find_for_provider(&provider)
            .expect("adapter should exist");

        assert_eq!(adapter.provider_type(), "openai_responses");
    }

    #[test]
    fn openai_chat_falls_back_to_chat_adapter() {
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(TestAdapter("deepseek")));

        let provider = provider("local-chat", json!({ "apiFormat": "openai_chat" }));
        let adapter = registry
            .find_for_provider(&provider)
            .expect("adapter should exist");

        assert_eq!(adapter.provider_type(), "deepseek");
    }
}
