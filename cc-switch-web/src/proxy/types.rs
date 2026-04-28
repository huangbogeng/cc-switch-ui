//! Proxy types

use std::net::SocketAddr;
use std::time::Instant;

/// Proxy server configuration
#[derive(Clone, Debug)]
pub struct ProxyConfig {
    pub listen_addr: SocketAddr,
    pub upstream_url: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:15721".parse().unwrap(),
            upstream_url: "https://api.openai.com".to_string(),
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