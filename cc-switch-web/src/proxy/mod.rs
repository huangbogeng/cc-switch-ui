//! Proxy module for cc-switch-web

pub mod forwarder;
pub mod handlers;
pub mod server;
pub mod types;

pub use handlers::{proxy_set_target, proxy_start, proxy_status, proxy_stop, proxy_target};
pub use server::ProxyServer;
pub use types::ProxyConfig;
