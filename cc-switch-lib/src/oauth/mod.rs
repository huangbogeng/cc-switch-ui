//! OAuth authentication modules
//!
//! - copilot: GitHub Copilot authentication
//! - codex: OpenAI Codex/ChatGPT OAuth
//! - http_client: Shared HTTP client with proxy and timeout support

pub mod codex;
pub mod copilot;
pub mod http_client;

pub use codex::*;
pub use copilot::*;
pub use http_client::{new_http_client, new_http_client_with_proxy};
