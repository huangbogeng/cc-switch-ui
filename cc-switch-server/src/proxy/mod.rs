//! Proxy module for cc-switch-server

pub mod adapters;
pub mod circuit_breaker;
pub mod failover_switch;
pub mod forwarder;
pub mod handlers;
pub mod headers;
pub mod provider_router;
mod reasoning_bridge;
pub mod responses_aggregate;
mod responses_contract;
pub mod server;
pub mod session;
pub mod streaming;
pub mod transform_chat;
pub mod transform_content;
pub mod transform_responses;
pub mod types;

pub use handlers::{proxy_set_target, proxy_start, proxy_status, proxy_stop, proxy_target};
pub use server::ProxyServer;
pub use types::ProxyConfig;
