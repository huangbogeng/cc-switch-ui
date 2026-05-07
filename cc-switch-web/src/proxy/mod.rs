//! Proxy module for cc-switch-web

pub mod adapters;
pub mod forwarder;
pub mod handlers;
pub mod headers;
pub mod responses_aggregate;
pub mod server;
pub mod session;
pub mod streaming_responses;
pub mod transform_responses;
pub mod types;

pub use handlers::{proxy_set_target, proxy_start, proxy_status, proxy_stop, proxy_target};
pub use server::ProxyServer;
pub use types::ProxyConfig;
