//! Proxy types

use std::net::SocketAddr;
use std::time::Instant;

/// Proxy server configuration
#[derive(Clone, Debug)]
pub struct ProxyConfig {
    pub listen_addr: SocketAddr,
    pub upstream_url: String,
    pub http_proxy_url: Option<String>,
    pub prompt_cache_key: String,
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
            prompt_cache_key: "codex_oauth".to_string(),
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
