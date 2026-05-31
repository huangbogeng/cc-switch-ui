//! Proxy types

use std::net::SocketAddr;
use std::time::Instant;

/// Proxy server configuration
#[derive(Clone, Debug)]
pub struct ProxyConfig {
    pub listen_addr: SocketAddr,
    pub upstream_url: String,
    pub http_proxy_url: Option<String>,
    pub prompt_cache_key: Option<String>,
    pub prompt_cache_key_fallback: String,
    pub codex_fast_mode: bool,
    pub model_mapping: ModelMapping,
}

#[derive(Clone, Debug, Default)]
pub struct ModelMapping {
    pub default_model: Option<String>,
    pub haiku_model: Option<String>,
    pub sonnet_model: Option<String>,
    pub opus_model: Option<String>,
}

impl ModelMapping {
    pub fn map_model(&self, original_model: &str) -> String {
        let lower = original_model.to_ascii_lowercase();
        if lower.contains("haiku") {
            if let Some(model) = &self.haiku_model {
                return model.clone();
            }
        }
        if lower.contains("opus") {
            if let Some(model) = &self.opus_model {
                return model.clone();
            }
        }
        if lower.contains("sonnet") {
            if let Some(model) = &self.sonnet_model {
                return model.clone();
            }
        }
        self.default_model
            .clone()
            .unwrap_or_else(|| original_model.to_string())
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:15721".parse().unwrap(),
            upstream_url: "https://chatgpt.com/backend-api/codex/responses".to_string(),
            http_proxy_url: None,
            prompt_cache_key: None,
            prompt_cache_key_fallback: "codex_oauth".to_string(),
            codex_fast_mode: false,
            model_mapping: ModelMapping::default(),
        }
    }
}

/// Proxy server runtime status
#[derive(Clone, Debug, Default)]
pub struct ProxyStatus {
    pub running: bool,
    pub start_time: Option<Instant>,
    pub request_count: u64,
    pub listen_addr: Option<SocketAddr>,
}

impl ProxyStatus {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Shared state for an active proxy provider candidate.
#[derive(Clone)]
pub struct ProxyState {
    pub adapter: std::sync::Arc<dyn cc_switch_lib::providers::ProviderAdapter>,
    pub account_id: Option<String>,
    pub provider: cc_switch_lib::database::Provider,
    pub db: std::sync::Arc<cc_switch_lib::database::Database>,
    pub provider_id: String,
}

impl ProxyState {
    pub fn new(
        adapter: std::sync::Arc<dyn cc_switch_lib::providers::ProviderAdapter>,
        account_id: Option<String>,
        provider: cc_switch_lib::database::Provider,
        db: std::sync::Arc<cc_switch_lib::database::Database>,
        provider_id: String,
    ) -> Self {
        Self {
            adapter,
            account_id,
            provider,
            db,
            provider_id,
        }
    }
}
