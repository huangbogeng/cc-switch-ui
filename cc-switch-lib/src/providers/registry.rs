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
    /// then tries partial ID matching (for providers like "gemini-native" -> "gemini")
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
        None
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
