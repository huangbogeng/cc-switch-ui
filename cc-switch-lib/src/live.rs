//! Live config synchronization module
//!
//! Handles syncing provider settings to Claude Code's live config files.

use crate::config::{get_claude_settings_path, read_json_file, write_json_file};
use crate::error::AppError;
use serde_json::Value;
use std::path::PathBuf;

/// Get the path to the live Claude settings file
pub fn get_live_settings_path() -> PathBuf {
    get_claude_settings_path()
}

/// Remove internal-only fields before writing to Claude Code's live config.
///
/// These fields are used internally by cc-switch for routing and should never
/// appear in the Claude Code settings.json that the user or Claude reads.
pub fn sanitize_claude_settings_for_live(settings: &Value) -> Value {
    let mut v = settings.clone();
    if let Some(obj) = v.as_object_mut() {
        obj.remove("api_format");
        obj.remove("apiFormat");
        obj.remove("openrouter_compat_mode");
        obj.remove("openrouterCompatMode");
        obj.remove("provider_type");
        obj.remove("providerType");
        // Cross-module fields that should never leak into provider config
        obj.remove("mcpServers");
        obj.remove("hasCompletedOnboarding");
    }
    v
}

/// Read current live settings from disk as raw JSON
fn read_live_settings_raw() -> Result<Value, AppError> {
    let path = get_claude_settings_path();
    if !path.exists() {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    read_json_file(&path)
}

/// Check whether the live config is currently pointing at the local proxy.
///
/// Returns `true` if `ANTHROPIC_BASE_URL` points to `127.0.0.1` — meaning
/// the proxy has taken over and is responsible for routing API requests.
///
/// Previously required `ANTHROPIC_AUTH_TOKEN == "PROXY_MANAGED"` as well, but
/// that only holds for OAuth providers (Codex/Copilot). Direct API-key providers
/// (DeepSeek, MiniMax, etc.) keep their real key in the live config so the
/// proxy can forward it. Checking the base URL alone correctly detects takeover
/// for both provider types.
pub fn detect_takeover_in_live_config() -> bool {
    let settings = match read_live_settings_raw() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let env = settings.get("env").and_then(Value::as_object);
    let base_url = env
        .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
        .and_then(Value::as_str)
        .unwrap_or("");
    base_url.starts_with("http://127.0.0.1:")
}

/// Read current live settings and sanitize them for backfill storage.
///
/// This captures the user's current live config (which may include direct edits
/// to the Claude settings file) and prepares it to be saved as a provider's
/// settings_config. Without this backfill, user customizations made directly
/// in the live file would be lost on the next switch.
///
/// Returns the sanitized settings ready to store in a provider's DB record.
pub fn backfill_current_live_config() -> Result<Value, AppError> {
    let live_settings = read_live_settings_raw()?;
    Ok(sanitize_claude_settings_for_live(&live_settings))
}

/// Placeholder auth token written to the live config during proxy takeover.
/// The real credential is stored in the database, and the proxy injects it
/// during forwarding. The placeholder signals "managed by proxy" to any
/// tooling that inspects the live config.
pub const PROXY_TOKEN_PLACEHOLDER: &str = "PROXY_MANAGED";

/// Stable Claude role aliases written to the live config during proxy
/// takeover — mirrors upstream cc-switch.  Claude Code recognizes these
/// names and the proxy maps them to the real provider model on the fly.
pub const CLAUDE_TAKEOVER_HAIKU: &str = "claude-haiku-4-5";
pub const CLAUDE_TAKEOVER_SONNET: &str = "claude-sonnet-4-6";
pub const CLAUDE_TAKEOVER_OPUS: &str = "claude-opus-4-8";

/// Build the sanitized live-settings payload for a provider.
///
/// When `use_proxy` is true the env is rewritten to route all Claude Code
/// requests through the local proxy (matching upstream cc-switch behaviour).
/// Otherwise the provider's settings_config is passed through as-is.
pub fn settings_for_live(
    provider: &crate::database::Provider,
    proxy_port: u16,
    use_proxy: bool,
) -> Value {
    let mut settings = provider.settings_config.clone();

    if !settings.is_object() {
        settings = serde_json::json!({});
    }
    let root = settings
        .as_object_mut()
        .expect("settings should be normalized to object");
    let env = root
        .entry("env".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !env.is_object() {
        *env = serde_json::json!({});
    }
    let env = env
        .as_object_mut()
        .expect("settings env should be normalized to object");

    if use_proxy {
        env.clear();
        env.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            serde_json::json!(format!("http://127.0.0.1:{}", proxy_port)),
        );
        env.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            serde_json::json!(PROXY_TOKEN_PLACEHOLDER),
        );
        env.insert(
            "ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(),
            serde_json::json!(CLAUDE_TAKEOVER_HAIKU),
        );
        env.insert(
            "ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(),
            serde_json::json!(CLAUDE_TAKEOVER_SONNET),
        );
        env.insert(
            "ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(),
            serde_json::json!(CLAUDE_TAKEOVER_OPUS),
        );
    }

    settings
}

/// Write Claude settings to disk (atomic write)
fn write_live_settings_raw(settings: &Value) -> Result<(), AppError> {

    let path = get_claude_settings_path();
    write_json_file(&path, settings)
}

/// Apply provider settings to live Claude config (merge mode).
///
/// Reads the current `~/.claude/settings.json`, updates only the `env`
/// field with the provider's configuration, and writes back.
/// Other fields (project-level config, Claude Code settings, etc.) are
/// preserved. This avoids data loss when multiple modules share the file.
///
/// For proxy mode, callers should use `settings_for_live` first to rewrite
/// the env for proxy routing before calling this function.
pub fn apply_provider_to_live(settings_config: &serde_json::Value) -> Result<(), AppError> {
    let settings = sanitize_claude_settings_for_live(settings_config);
    let mut current = read_live_settings_raw()?;
    if let Some(current_obj) = current.as_object_mut() {
        if let Some(env) = settings.get("env") {
            current_obj.insert("env".into(), env.clone());
        }
    }
    write_live_settings_raw(&current)?;
    log::info!(
        "Applied provider settings to live Claude config at {:?}",
        get_live_settings_path()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn acquire_env_lock() -> std::sync::MutexGuard<'static, ()> {
        env_lock().lock().unwrap_or_else(|e| e.into_inner())
    }

    fn setup_test_home() -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let test_home = std::env::temp_dir().join(format!("cc-switch-live-test-{suffix}"));
        std::fs::create_dir_all(&test_home).expect("create test home");
        unsafe {
            std::env::set_var(
                "CC_SWITCH_TEST_HOME",
                test_home.to_string_lossy().to_string(),
            );
        }
        test_home
    }

    fn cleanup(test_home: std::path::PathBuf) {
        unsafe {
            std::env::remove_var("CC_SWITCH_TEST_HOME");
        }
        let _ = std::fs::remove_dir_all(test_home);
    }

    #[test]
    fn apply_provider_to_live_writes_env_directly() {
        let _guard = acquire_env_lock();
        let test_home = setup_test_home();

        let settings = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
                "ANTHROPIC_API_KEY": "sk-test-key",
                "ANTHROPIC_MODEL": "deepseek-v4-pro"
            }
        });
        apply_provider_to_live(&settings).expect("apply settings");

        let live = read_live_settings_raw().expect("read live settings");
        // No top-level fields extracted from env
        assert_eq!(live.get("base_url"), None);
        assert_eq!(live.get("api_key"), None);
        // Env content matches input
        assert_eq!(
            live.pointer("/env/ANTHROPIC_BASE_URL").and_then(Value::as_str),
            Some("https://api.deepseek.com/anthropic")
        );
        assert_eq!(
            live.pointer("/env/ANTHROPIC_API_KEY").and_then(Value::as_str),
            Some("sk-test-key")
        );

        cleanup(test_home);
    }

    #[test]
    fn apply_provider_to_live_strips_internal_fields() {
        let _guard = acquire_env_lock();
        let test_home = setup_test_home();

        // Pre-write a file with a field that should survive merge
        let existing = json!({
            "keep_this_field": "should survive"
        });
        write_live_settings_raw(&existing).expect("pre-write existing");

        let settings = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
                "ANTHROPIC_API_KEY": "sk-test-key"
            },
            "api_format": "openai_chat",
            "openrouter_compat_mode": true
        });
        apply_provider_to_live(&settings).expect("apply settings");

        let live = read_live_settings_raw().expect("read live settings");
        // Internal fields from input are sanitized — not in the output
        assert_eq!(live.get("api_format"), None);
        assert_eq!(live.get("openrouter_compat_mode"), None);
        // Non-env fields from existing file survive the merge
        assert_eq!(
            live.get("keep_this_field").and_then(Value::as_str),
            Some("should survive")
        );
        // Env content matches input
        assert_eq!(
            live.pointer("/env/ANTHROPIC_BASE_URL").and_then(Value::as_str),
            Some("https://api.deepseek.com/anthropic")
        );

        cleanup(test_home);
    }

    #[test]
    fn apply_provider_to_live_merges_env_only() {
        let _guard = acquire_env_lock();
        let test_home = setup_test_home();

        // Pre-write a file with non-env root fields that must survive
        let prewrite = json!({
            "project": "my-project",
            "theme": "dark"
        });
        write_live_settings_raw(&prewrite).expect("pre-write existing");

        // First write: adds env, should preserve project/theme
        let first = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "http://127.0.0.1:15721",
                "ANTHROPIC_AUTH_TOKEN": "PROXY_MANAGED"
            }
        });
        apply_provider_to_live(&first).expect("apply first");

        // Second write: only env changes, project/theme should survive
        let second = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
                "ANTHROPIC_API_KEY": "real-key"
            }
        });
        apply_provider_to_live(&second).expect("apply second");

        let live = read_live_settings_raw().expect("read live settings");
        // project and theme from pre-write must survive
        assert_eq!(
            live.get("project").and_then(Value::as_str),
            Some("my-project")
        );
        assert_eq!(
            live.get("theme").and_then(Value::as_str),
            Some("dark")
        );
        // Env is replaced by second write
        assert_eq!(
            live.pointer("/env/ANTHROPIC_BASE_URL").and_then(Value::as_str),
            Some("https://api.deepseek.com/anthropic")
        );
        assert_eq!(
            live.pointer("/env/ANTHROPIC_API_KEY").and_then(Value::as_str),
            Some("real-key")
        );
        // PROXY_MANAGED from first write env is gone (env was replaced)
        assert_eq!(live.pointer("/env/ANTHROPIC_AUTH_TOKEN"), None);

        cleanup(test_home);
    }

    fn provider_with_env(env: serde_json::Value) -> crate::database::Provider {
        crate::database::Provider {
            id: "minimax".to_string(),
            name: "MiniMax".to_string(),
            settings_config: json!({ "env": env }),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            icon: None,
            icon_color: None,
            meta: json!({ "providerType": "minimax" }),
            in_failover_queue: false,
        }
    }

    #[test]
    fn proxy_mode_writes_minimal_takeover_env() {
        let provider = provider_with_env(json!({
            "ANTHROPIC_BASE_URL": "https://api.minimaxi.com/v1",
            "ANTHROPIC_AUTH_TOKEN": "real-token",
            "API_TIMEOUT_MS": "3000000"
        }));

        let settings = settings_for_live(&provider, 15721, true);
        let env = settings
            .get("env")
            .and_then(|value| value.as_object())
            .unwrap();

        assert_eq!(env.len(), 5, "takeover env must have exactly 5 keys");
        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()),
            Some("http://127.0.0.1:15721")
        );
        assert_eq!(
            env.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str()),
            Some("PROXY_MANAGED")
        );
        assert_eq!(
            env.get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
                .and_then(|v| v.as_str()),
            Some("claude-haiku-4-5")
        );
        assert_eq!(
            env.get("ANTHROPIC_DEFAULT_SONNET_MODEL")
                .and_then(|v| v.as_str()),
            Some("claude-sonnet-4-6")
        );
        assert_eq!(
            env.get("ANTHROPIC_DEFAULT_OPUS_MODEL")
                .and_then(|v| v.as_str()),
            Some("claude-opus-4-8")
        );
        assert_eq!(env.get("API_TIMEOUT_MS"), None);
        assert_eq!(env.get("ANTHROPIC_MODEL"), None);
    }

    #[test]
    fn restore_mode_preserves_direct_provider_settings() {
        let provider = provider_with_env(json!({
            "ANTHROPIC_BASE_URL": "https://api.minimaxi.com/v1",
            "ANTHROPIC_AUTH_TOKEN": "real-token"
        }));

        let settings = settings_for_live(&provider, 15721, false);
        let env = settings
            .get("env")
            .and_then(|value| value.as_object())
            .unwrap();

        assert_eq!(
            env.get("ANTHROPIC_BASE_URL")
                .and_then(|value| value.as_str()),
            Some("https://api.minimaxi.com/v1")
        );
        assert_eq!(
            env.get("ANTHROPIC_AUTH_TOKEN")
                .and_then(|value| value.as_str()),
            Some("real-token")
        );
    }
}
