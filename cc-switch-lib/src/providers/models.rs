//! Domain models for provider management
//!
//! Types for AppType, ProviderMeta, switch results, and related enums.
//! These are the business domain types used by the database, live config,
//! and switch flow layers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// AppType
// ============================================================================

/// Supported application types.
///
/// Currently only `ClaudeCode` is active. `Codex` and `OpenCode` are reserved
/// for future use — they can be serialised/deserialised but business logic
/// only handles `ClaudeCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppType {
    ClaudeCode,
    #[serde(alias = "codex")]
    Codex,
    #[serde(alias = "opencode")]
    OpenCode,
}

impl AppType {
    pub fn as_str(&self) -> &str {
        match self {
            AppType::ClaudeCode => "claude_code",
            AppType::Codex => "codex",
            AppType::OpenCode => "opencode",
        }
    }
}

impl std::str::FromStr for AppType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_lowercase();
        match normalized.as_str() {
            "claude_code" | "claudecode" | "claude" => Ok(AppType::ClaudeCode),
            "codex" => Ok(AppType::Codex),
            "opencode" => Ok(AppType::OpenCode),
            _ => Err(format!("Unknown app type: {s}")),
        }
    }
}

// ============================================================================
// SwitchResult
// ============================================================================

/// Result of a provider switch operation, including any non-fatal warnings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SwitchResult {
    pub warnings: Vec<String>,
}

// ============================================================================
// ProviderMeta
// ============================================================================

/// Provider metadata — stored as JSON in the `meta` column.
///
/// Mirrors the original cc-switch `ProviderMeta` struct. All fields are
/// optional to support partial deserialisation of legacy data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderMeta {
    /// Custom endpoint list, keyed by URL.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom_endpoints: HashMap<String, CustomEndpoint>,

    /// Whether to apply the common config snippet when writing live config.
    #[serde(
        rename = "commonConfigEnabled",
        skip_serializing_if = "Option::is_none"
    )]
    pub common_config_enabled: Option<bool>,

    /// Usage-query script configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_script: Option<UsageScript>,

    /// Cost multiplier for usage pricing.
    #[serde(rename = "costMultiplier", skip_serializing_if = "Option::is_none")]
    pub cost_multiplier: Option<String>,

    /// API format for proxy routing (anthropic, openai_chat, openai_responses, google).
    #[serde(rename = "apiFormat", skip_serializing_if = "Option::is_none")]
    pub api_format: Option<String>,

    /// Generic auth binding (provider_config / managed_account).
    #[serde(rename = "authBinding", skip_serializing_if = "Option::is_none")]
    pub auth_binding: Option<AuthBinding>,

    /// Provider type identifier (e.g. "github_copilot", "codex_oauth").
    #[serde(rename = "providerType", skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<String>,

    /// In additive-mode apps: whether this provider has been written to live config.
    #[serde(rename = "liveConfigManaged", skip_serializing_if = "Option::is_none")]
    pub live_config_managed: Option<bool>,

    /// API key field name ("ANTHROPIC_AUTH_TOKEN" or "ANTHROPIC_API_KEY").
    #[serde(rename = "apiKeyField", skip_serializing_if = "Option::is_none")]
    pub api_key_field: Option<String>,

    /// Whether base_url is a full endpoint URL (don't append path).
    #[serde(rename = "isFullUrl", skip_serializing_if = "Option::is_none")]
    pub is_full_url: Option<bool>,

    /// Prompt cache key override for OpenAI Responses-compatible endpoints.
    #[serde(rename = "promptCacheKey", skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,

    /// Codex OAuth FAST mode (injects `service_tier = "priority"`).
    #[serde(rename = "codexFastMode", skip_serializing_if = "Option::is_none")]
    pub codex_fast_mode: Option<bool>,

    /// Icon identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Icon color in hex format.
    #[serde(rename = "iconColor", skip_serializing_if = "Option::is_none")]
    pub icon_color: Option<String>,

    /// Snippet category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Whether this provider is an official seed provider.
    #[serde(default)]
    pub official: bool,
}

// ============================================================================
// Supporting types
// ============================================================================

/// A custom endpoint entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEndpoint {
    pub url: String,
    #[serde(rename = "addedAt")]
    pub added_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastUsed")]
    pub last_used: Option<i64>,
}

/// Generic auth binding — associates a provider with an OAuth / managed account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthBinding {
    /// Auth source identifier.
    #[serde(default)]
    pub source: AuthBindingSource,
    /// Managed auth provider name (e.g. "github_copilot").
    #[serde(rename = "authProvider", skip_serializing_if = "Option::is_none")]
    pub auth_provider: Option<String>,
    /// Managed account ID; empty means use the default account.
    #[serde(rename = "accountId", skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
}

/// Source of auth credentials.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthBindingSource {
    /// Read credentials from the provider's own config.
    #[default]
    ProviderConfig,
    /// Use a managed account (e.g. GitHub Copilot OAuth).
    ManagedAccount,
}

/// Usage-query script configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageScript {
    pub enabled: bool,
    pub language: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(rename = "apiKey", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(rename = "baseUrl", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(rename = "accessToken", skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(rename = "templateType", skip_serializing_if = "Option::is_none")]
    pub template_type: Option<String>,
    #[serde(rename = "autoQueryInterval", skip_serializing_if = "Option::is_none")]
    pub auto_query_interval: Option<u64>,
    #[serde(rename = "codingPlanProvider", skip_serializing_if = "Option::is_none")]
    pub coding_plan_provider: Option<String>,
}
