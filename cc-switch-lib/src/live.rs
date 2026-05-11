//! Live config synchronization module
//!
//! Handles syncing provider settings to Claude Code's live config files.

use crate::config::{get_claude_settings_path, read_json_file, write_json_file};
use crate::error::AppError;
use serde_json::Value;
use std::collections::BTreeMap;
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
/// Returns `true` if the live config has `ANTHROPIC_AUTH_TOKEN = "PROXY_MANAGED"`
/// and `ANTHROPIC_BASE_URL` points to `127.0.0.1` — meaning the proxy has taken
/// over and is responsible for routing API requests.
///
/// Checks inside the `env` object where provider settings are stored.
pub fn detect_takeover_in_live_config() -> bool {
    let settings = match read_live_settings_raw() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let env = settings.get("env").and_then(Value::as_object);
    let auth_token = env
        .and_then(|e| e.get("ANTHROPIC_AUTH_TOKEN"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let base_url = env
        .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
        .and_then(Value::as_str)
        .unwrap_or("");
    auth_token == "PROXY_MANAGED" && base_url.starts_with("http://127.0.0.1:")
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
}
