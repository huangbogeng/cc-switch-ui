//! cc-switch-lib
//!
//! Core library for cc-switch providing:
//! - Database persistence
//! - OAuth authentication

/// Default application type used across the workspace.
/// Mirrors `providers::AppType::ClaudeCode.as_str()`.
pub const DEFAULT_APP_TYPE: &str = "claude_code";

pub mod app_store;
pub mod config;
pub mod database;
pub mod error;
pub mod live;
pub mod mcp;
pub mod oauth;
pub mod providers;
pub mod settings;
pub mod skills;
pub mod usage;

pub use database::{Database, FailoverQueueItem, McpServerRecord, ProxyConfig, ProxyType, SkillRecord};
pub use error::AppError;
pub use oauth::{CodexOAuthManager, CopilotAuthManager};
pub use providers::{ProviderAdapter, ProviderRegistry};
