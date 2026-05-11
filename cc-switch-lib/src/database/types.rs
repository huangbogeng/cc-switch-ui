//! Database types — shared data structures used across the codebase.

use serde::{Deserialize, Serialize};

/// Provider struct matching frontend Provider interface
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub settings_config: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_color: Option<String>,
    #[serde(default)]
    pub meta: serde_json::Value,
    #[serde(default)]
    pub in_failover_queue: bool,
}

/// Proxy configuration for OAuth authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub enabled: bool,
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub auto_failover_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyType {
    Http,
    Socks5,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            proxy_type: ProxyType::Http,
            host: String::new(),
            port: 10809,
            auto_failover_enabled: false,
        }
    }
}

/// MCP server record for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerRecord {
    pub id: String,
    pub name: String,
    pub server_spec: serde_json::Value,
    pub app_type: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool { true }

/// Skill record for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRecord {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub directory: String,
    pub app_type: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    #[serde(default)]
    pub installed_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readme_url: Option<String>,
}

/// Usage record for database storage
#[derive(Debug, Clone)]
pub struct UsageRecord {
    pub provider_id: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: Option<i64>,
    pub request_timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct ProxyRequestLogRecord {
    pub app_type: String,
    pub provider_id: String,
    pub request_path: String,
    pub request_model: Option<String>,
    pub status_code: Option<i32>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProxyRequestLogEntry {
    pub app_type: String,
    pub provider_id: String,
    pub request_path: String,
    pub request_model: Option<String>,
    pub status_code: Option<i32>,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: i64,
}

/// Live backup record for proxy takeover detection/restore
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveBackup {
    pub app_type: String,
    pub provider_id: String,
    pub original_config: String,
    pub created_at: String,
}

/// Usage summary by provider
#[derive(Debug, Clone, Serialize)]
pub struct ProviderUsageSummary {
    pub provider_id: String,
    pub model: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub request_count: i64,
}

/// Daily usage aggregate
#[derive(Debug, Clone, Serialize)]
pub struct DailyUsage {
    pub day: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub request_count: i64,
}

/// Failover queue item for external reference
pub struct FailoverQueueItem;
