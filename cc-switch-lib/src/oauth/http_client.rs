//! HTTP client utilities for OAuth modules
//!
//! Provides a pre-configured HTTP client with:
//! - Proxy support from environment variables (HTTPS_PROXY, https_proxy, ALL_PROXY, all_proxy)
//! - Or explicit proxy configuration via ProxyConfig
//! - Connection timeout: 30 seconds
//! - Read timeout: 60 seconds

use crate::database::{ProxyConfig, ProxyType};
use reqwest::Client;

/// Build an HTTP client with proxy support and reasonable timeouts.
///
/// Reads standard proxy environment variables:
/// - `HTTPS_PROXY` / `https_proxy` / `ALL_PROXY` / `all_proxy`
///
/// Falls back to direct connection if no proxy is set.
pub fn new_http_client() -> Result<Client, reqwest::Error> {
    let mut builder = Client::builder()
        .use_rustls_tls()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(60));

    // Check for proxy environment variables
    let proxy_url = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .or_else(|_| std::env::var("ALL_PROXY"))
        .or_else(|_| std::env::var("all_proxy"))
        .ok();

    if let Some(proxy_url) = proxy_url.as_deref().filter(|s| !s.trim().is_empty()) {
        log::info!("[HTTP Client] Using proxy from environment: {}", proxy_url);
        // Try HTTP proxy first, then SOCKS5
        if let Ok(proxy) = reqwest::Proxy::http(proxy_url) {
            builder = builder.proxy(proxy);
            log::info!("[HTTP Client] Configured as HTTP proxy");
        } else if let Ok(proxy) = reqwest::Proxy::all(proxy_url) {
            builder = builder.proxy(proxy);
            log::info!("[HTTP Client] Configured as SOCKS5 proxy");
        } else {
            log::warn!("[HTTP Client] Failed to parse proxy URL '{}' as HTTP or SOCKS5", proxy_url);
        }
    }

    builder.build()
}

/// Build an HTTP client with explicit proxy configuration.
pub fn new_http_client_with_proxy(proxy_config: &ProxyConfig) -> Result<Client, reqwest::Error> {
    let mut builder = Client::builder()
        .use_rustls_tls()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(60));

    if proxy_config.enabled {
        let proxy_type_str = match proxy_config.proxy_type {
            ProxyType::Http => "http",
            ProxyType::Socks5 => "socks5",
        };
        let proxy_url = format!("{}://{}:{}", proxy_type_str, proxy_config.host, proxy_config.port);
        log::info!("[HTTP Client] Using proxy from config: {}", proxy_url);

        match proxy_config.proxy_type {
            ProxyType::Http => {
                if let Ok(proxy) = reqwest::Proxy::http(&proxy_url) {
                    builder = builder.proxy(proxy);
                    log::info!("[HTTP Client] Configured as HTTP proxy");
                } else {
                    log::warn!("[HTTP Client] Failed to configure HTTP proxy");
                }
            }
            ProxyType::Socks5 => {
                if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
                    builder = builder.proxy(proxy);
                    log::info!("[HTTP Client] Configured as SOCKS5 proxy");
                } else {
                    log::warn!("[HTTP Client] Failed to configure SOCKS5 proxy");
                }
            }
        }
    }

    builder.build()
}
