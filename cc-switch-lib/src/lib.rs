//! cc-switch-lib
//!
//! Core library for cc-switch providing:
//! - Database persistence
//! - OAuth authentication

#![allow(unused)]

pub mod app_store;
pub mod config;
pub mod database;
pub mod error;
pub mod live;
pub mod oauth;
pub mod settings;

pub use database::{Database, FailoverQueueItem, ProxyConfig, ProxyType};
pub use error::AppError;
pub use oauth::{CodexOAuthManager, CopilotAuthManager};
