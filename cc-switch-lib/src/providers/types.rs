//! Shared types for provider adapters

use crate::database::UsageRecord;
use bytes::Bytes;

/// Result type for provider operations
pub type ProviderResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Auth token with optional metadata
#[derive(Debug, Clone)]
pub struct AuthToken {
    pub token: String,
    pub expires_at_ms: Option<i64>,
}

/// Request transform input
#[derive(Debug, Clone)]
pub struct TransformInput {
    pub body: serde_json::Value,
    pub upstream_url: String,
    pub http_proxy_url: Option<String>,
    pub prompt_cache_key: Option<String>,
    pub requested_stream: bool,
}

/// Request transform output
#[derive(Debug, Clone)]
pub struct TransformOutput {
    pub body: serde_json::Value,
    pub upstream_url: String,
    pub headers: Vec<(String, String)>,
    pub method: String,
}

/// Usage parse result
#[derive(Debug)]
pub struct UsageParseResult {
    pub record: Option<UsageRecord>,
    pub body: Bytes,
}
