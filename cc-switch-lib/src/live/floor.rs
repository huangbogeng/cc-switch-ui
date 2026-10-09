//! Key-field ("floor") and provider-exclusive definitions for Claude Code.
//!
//! Adapted from upstream cc-switch `src-tauri/src/live/floor.rs` (Claude subset).
//!
//! Key fields answer four questions: where requests go, how they authenticate,
//! which model names to use, and which protocol to speak. They are fully owned
//! by the provider: on switch, every key field is cleared and rewritten from the
//! target provider. Everything else belongs to the user or the client — this
//! tool never writes or deletes it.
//!
//! Prefix matching is only used where the entire prefix is connection/auth
//! related (`ANTHROPIC_*`, `AWS_*`, `VERTEX_REGION_*`). A key that is missed by
//! the lists is treated as a user key and preserved — the failure mode is safe.
//!
//! Provider-exclusive fields are compatibility switches and window values: they
//! are written on switch-in, and on switch-out only removed when the value in
//! the live file still matches what the previous provider wrote (i.e. the user
//! never edited them).

/// Claude Code protocol selectors.
///
/// `CLAUDE_CODE_USE_` must NOT be prefix-matched: the same prefix covers
/// provider-unrelated feature switches such as `USE_POWERSHELL_TOOL`,
/// `USE_NATIVE_FILE_SEARCH`, `USE_COWORK_PLUGINS`, and `USE_CCR_V2`. Every
/// `CLAUDE_CODE_USE_*` key that presets may write has to be listed here.
pub const CLAUDE_PROTOCOL_SELECTORS: &[&str] = &[
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
    "CLAUDE_CODE_USE_GATEWAY",
    "CLAUDE_CODE_USE_MANTLE",
    "CLAUDE_CODE_USE_ANTHROPIC_AWS",
    "CLAUDE_CODE_USE_ANTHROPIC_GOOGLE_CLOUD",
];

/// `env` prefixes that are entirely connection/auth related: `ANTHROPIC_*` is
/// the endpoint, credentials, per-tier model names, and custom headers;
/// `AWS_*` is Bedrock region and credentials; `VERTEX_REGION_*` is Vertex
/// per-model regions.
pub const CLAUDE_FLOOR_ENV_PREFIXES: &[&str] = &["ANTHROPIC_", "AWS_", "VERTEX_REGION_"];

/// Named key fields in `env` (beyond the protocol selectors).
pub const CLAUDE_FLOOR_ENV_KEYS: &[&str] = &[
    "CLAUDE_CODE_SUBAGENT_MODEL",
    "CLAUDE_CODE_SUBAGENT_MODEL_FORCE",
    "CLOUD_ML_REGION",
    // Vertex credential path.
    "GOOGLE_APPLICATION_CREDENTIALS",
    // Subscription account long-lived tokens and companions.
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
    "CLAUDE_CODE_OAUTH_SCOPES",
    // Companion of the top-level apiKeyHelper.
    "CLAUDE_CODE_API_KEY_HELPER_TTL_MS",
];

/// Whether a key in Claude Code `settings.json` `env` is a key field.
pub fn claude_floor_env(key: &str) -> bool {
    CLAUDE_FLOOR_ENV_PREFIXES
        .iter()
        .any(|prefix| key.starts_with(prefix))
        || CLAUDE_PROTOCOL_SELECTORS.contains(&key)
        || CLAUDE_FLOOR_ENV_KEYS.contains(&key)
        || (key.starts_with("CLAUDE_CODE_SKIP_") && key.ends_with("_AUTH"))
}

/// Provider-exclusive fields in Claude Code `env`.
///
/// Written on switch-in; on switch-out removed only when unchanged from what
/// the previous provider wrote. They are not key fields because the user may
/// also set them globally.
pub const CLAUDE_EXCLUSIVE_ENV: &[&str] = &[
    // AtlasCloud, Soshow, etc. reject experimental beta headers.
    "CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS",
    // DeepSeek and friends strictly validate tool schemas; Artifact tools 400.
    "CLAUDE_CODE_DISABLE_ARTIFACT",
    // Off by default for third-party endpoints.
    "ENABLE_TOOL_SEARCH",
    // Documented compatibility options for proxies, gateways, third parties.
    "CLAUDE_CODE_DISABLE_THINKING",
    "DISABLE_INTERLEAVED_THINKING",
    "CLAUDE_CODE_ALWAYS_ENABLE_EFFORT",
    "CLAUDE_CODE_EFFORT_LEVEL",
    "CLAUDE_CODE_EXTRA_BODY",
    "CLAUDE_CODE_ENABLE_FINE_GRAINED_TOOL_STREAMING",
    // The auto-mode server-side classifier is only supported on the official
    // endpoint; gateway setups must set 0 or sessions get stuck on a blocking
    // prompt (official compatibility option for proxies and gateways).
    "CLAUDE_CODE_AUTO_MODE_SERVER",
    // Window values derived from the upstream model's window.
    "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
    "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
    "CLAUDE_CODE_MAX_OUTPUT_TOKENS",
    "CLAUDE_CODE_DISABLE_1M_CONTEXT",
    "CLAUDE_CODE_DISABLE_UNKNOWN_MODEL_WINDOW_ENFORCEMENT",
    // Fetch the model list from ANTHROPIC_BASE_URL: gateways need it, and
    // users may also set it globally.
    "CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY",
];

pub fn claude_exclusive_env(key: &str) -> bool {
    CLAUDE_EXCLUSIVE_ENV.contains(&key)
}

/// Whether this tool manages the key at all (key field or exclusive field).
///
/// Keys outside this set are user keys: preserved in the live file on every
/// write, and never captured into provider records.
pub fn claude_owned_env(key: &str) -> bool {
    claude_floor_env(key) || claude_exclusive_env(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_covers_connection_keys() {
        for key in [
            "ANTHROPIC_BASE_URL",
            "ANTHROPIC_AUTH_TOKEN",
            "ANTHROPIC_DEFAULT_HAIKU_MODEL",
            "ANTHROPIC_CUSTOM_HEADERS",
            "CLAUDE_CODE_USE_BEDROCK",
            "CLAUDE_CODE_SKIP_BEDROCK_AUTH",
            "AWS_REGION",
            "AWS_BEARER_TOKEN_BEDROCK",
            "VERTEX_REGION_CLAUDE_4_5_SONNET",
            "CLOUD_ML_REGION",
            "GOOGLE_APPLICATION_CREDENTIALS",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "CLAUDE_CODE_SUBAGENT_MODEL",
        ] {
            assert!(claude_floor_env(key), "{key} should be a key field");
        }
    }

    #[test]
    fn floor_leaves_feature_switches_alone() {
        for key in [
            "CLAUDE_CODE_USE_POWERSHELL_TOOL",
            "CLAUDE_CODE_USE_NATIVE_FILE_SEARCH",
            "CLAUDE_CODE_USE_COWORK_PLUGINS",
            "CLAUDE_CODE_USE_CCR_V2",
            "CLAUDE_CODE_SKIP_PROMPT_HISTORY",
            "API_TIMEOUT_MS",
            "DISABLE_TELEMETRY",
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
            // Exclusive fields are not key fields.
            "CLAUDE_CODE_DISABLE_ARTIFACT",
            "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
            "CLAUDE_CODE_AUTO_MODE_SERVER",
        ] {
            assert!(!claude_floor_env(key), "{key} must stay a user key");
        }
    }

    #[test]
    fn exclusive_fields_never_overlap_key_fields() {
        for key in CLAUDE_EXCLUSIVE_ENV {
            assert!(!claude_floor_env(key), "{key} is listed twice");
        }
    }

    #[test]
    fn owned_is_the_union() {
        assert!(claude_owned_env("ANTHROPIC_BASE_URL"));
        assert!(claude_owned_env("CLAUDE_CODE_AUTO_MODE_SERVER"));
        assert!(!claude_owned_env("DISABLE_TELEMETRY"));
    }
}
