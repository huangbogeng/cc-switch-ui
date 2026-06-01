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
mod model_fetch;
mod models;
mod registry;
mod schema;
mod types;

pub use adapter::{BoxFuture, ProviderAdapter};
pub use error::ProviderError;
pub use model_fetch::{
    build_models_url_candidates, detect_endpoint_type, fetch_models, DetectedApiFormat,
    EndpointDetectionResult, EndpointProbeResult, FetchedModel,
};
pub use models::{
    AppType, AuthBinding, AuthBindingSource, CustomEndpoint, ProviderMeta, SwitchResult,
    UsageScript,
};
pub use registry::ProviderRegistry;
pub use schema::{
    normalize_provider_schema, provider_allows_empty_api_key, resolve_managed_account_id,
    resolve_provider_api_key, resolve_provider_api_key_field, ApiKeyField,
};
pub use types::{
    AuthInfo, AuthStrategy, StreamingResponseFormat, TransformInput, TransformOutput,
    UsageParseResult,
};
