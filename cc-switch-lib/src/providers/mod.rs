//! Provider adapter module
//!
//! Provides a generic interface for different API provider types,
//! domain models for provider management, and the adapter registry.
//!
//! Architecture:
//! - `adapter.rs` - ProviderAdapter trait definition
//! - `types.rs` - Shared types for the proxy adapter layer
//! - `models.rs` - Domain models (AppType, ProviderMeta, SwitchResult, etc.)
//! - `error.rs` - Error types
//! - `registry.rs` - ProviderRegistry for adapter lookup
//!
//! Concrete adapter implementations are in `cc-switch-server/src/proxy/adapters/`

mod adapter;
mod error;
mod models;
mod registry;
mod types;

pub use adapter::{BoxFuture, ProviderAdapter};
pub use error::ProviderError;
pub use models::{
    AppType, AuthBinding, AuthBindingSource, CustomEndpoint, ProviderMeta, SwitchResult,
    UsageScript,
};
pub use registry::ProviderRegistry;
pub use types::{
    AuthInfo, AuthStrategy, StreamingResponseFormat, TransformInput, TransformOutput,
    UsageParseResult,
};
