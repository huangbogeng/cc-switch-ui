//! Provider adapter module
//!
//! Provides a generic interface for different API provider types.
//! Each adapter handles authentication, request/response transformation,
//! and usage parsing for a specific provider type.
//!
//! Architecture:
//! - `adapter.rs` - ProviderAdapter trait definition
//! - `types.rs` - Shared types (AuthInfo, AuthStrategy, TransformInput, TransformOutput)
//! - `error.rs` - Error types
//! - `registry.rs` - ProviderRegistry for adapter lookup
//!
//! Concrete adapter implementations are in `cc-switch-web/src/proxy/adapters/`

mod adapter;
mod error;
mod registry;
mod types;

pub use adapter::{BoxFuture, ProviderAdapter};
pub use error::ProviderError;
pub use registry::ProviderRegistry;
pub use types::{
    AuthInfo, AuthStrategy, StreamingResponseFormat, TransformInput, TransformOutput,
    UsageParseResult,
};
