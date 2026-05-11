//! Shared types for provider adapters

use crate::database::UsageRecord;
use bytes::Bytes;

/// Authentication strategy used to build upstream request headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStrategy {
    /// Anthropic-compatible API key: `x-api-key: <secret>`.
    Anthropic,
    /// Bearer token for Claude relay services.
    ClaudeAuth,
    /// Generic bearer token: `Authorization: Bearer <secret>`.
    Bearer,
    /// Google API key: `x-goog-api-key: <secret>`.
    GoogleApiKey,
    /// Google OAuth token plus Gemini CLI client marker.
    GoogleOAuth,
    /// GitHub Copilot token plus Copilot client fingerprint headers.
    GitHubCopilot,
    /// ChatGPT/Codex OAuth access token plus Codex-specific headers.
    CodexOAuth,
}

/// Streaming upstream response format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingResponseFormat {
    /// Anthropic Messages SSE, already suitable for Claude-compatible clients.
    Anthropic,
    /// OpenAI Chat Completions SSE.
    OpenAIChat,
    /// OpenAI Responses SSE.
    OpenAIResponses,
}

/// Authentication data resolved from provider configuration or OAuth managers.
#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub secret: String,
    pub strategy: AuthStrategy,
    pub access_token: Option<String>,
}

impl AuthInfo {
    pub fn new(secret: String, strategy: AuthStrategy) -> Self {
        Self {
            secret,
            strategy,
            access_token: None,
        }
    }

    pub fn with_access_token(secret: String, access_token: String) -> Self {
        Self {
            secret,
            strategy: AuthStrategy::GoogleOAuth,
            access_token: Some(access_token),
        }
    }
}

/// Request transform input
#[derive(Debug, Clone)]
pub struct TransformInput {
    pub body: serde_json::Value,
    pub upstream_url: String,
    pub prompt_cache_key: Option<String>,
    pub requested_stream: bool,
    pub codex_fast_mode: bool,
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
