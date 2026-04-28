//! OAuth authentication modules
//!
//! - copilot_auth: GitHub Copilot authentication
//! - codex_oauth_auth: OpenAI Codex/ChatGPT OAuth

pub mod copilot_auth;
pub mod codex_oauth_auth;

pub use copilot_auth::*;
pub use codex_oauth_auth::*;
