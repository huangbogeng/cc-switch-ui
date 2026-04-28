//! Proxy module for cc-switch-web

pub mod forwarder;
pub mod handlers;
pub mod server;
pub mod types;

pub use handlers::{proxy_start, proxy_stop, proxy_status};
pub use server::ProxyServer;
pub use types::ProxyConfig;