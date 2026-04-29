//! OAuth authentication modules
//!
//! - copilot_auth: GitHub Copilot authentication
//! - codex_oauth_auth: OpenAI Codex/ChatGPT OAuth
//! - http_client: Shared HTTP client with proxy and timeout support

pub mod copilot_auth;
pub mod codex_oauth_auth;
pub mod http_client;

pub use copilot_auth::*;
pub use codex_oauth_auth::*;
pub use http_client::{new_http_client, new_http_client_with_proxy};
