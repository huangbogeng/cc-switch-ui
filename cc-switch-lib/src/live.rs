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

/// Read current Claude settings from disk as raw JSON
fn read_live_settings_raw() -> Result<Value, AppError> {
    let path = get_claude_settings_path();
    if !path.exists() {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    read_json_file(&path)
}

/// Write Claude settings to disk (atomic write)
fn write_live_settings_raw(settings: &Value) -> Result<(), AppError> {
    let path = get_claude_settings_path();
    write_json_file(&path, settings)
}

/// Apply provider settings to live Claude config
///
/// We only update specific fields:
/// - `api_key` (from ANTHROPIC_AUTH_TOKEN in env)
/// - `base_url` (from ANTHROPIC_BASE_URL in env)
/// - `env` (merged from provider's settings_config.env)
///
/// Other existing fields (hooks, enabledPlugins, etc.) are preserved.
pub fn apply_provider_to_live(settings_config: &serde_json::Value) -> Result<(), AppError> {
    let mut settings = read_live_settings_raw()?;

    // Get provider's env object
    let provider_env = settings_config
        .get("env")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    // Ensure settings is an object
    if !settings.is_object() {
        settings = Value::Object(serde_json::Map::new());
    }

    // Get or create env object
    let env_obj = if let Some(Value::Object(ref mut m)) = settings.get_mut("env") {
        m.clone()
    } else {
        serde_json::Map::new()
    };

    // Merge provider env into a new map
    let mut merged_env: serde_json::Map<String, Value> = env_obj;

    // Hard refresh provider-related keys to avoid cross-provider residue.
    // Keep unrelated env entries (plugins/hook/runtime settings), but remove
    // all ANTHROPIC_* keys before writing provider-specific values.
    let stale_keys: Vec<String> = merged_env
        .keys()
        .filter(|k| k.starts_with("ANTHROPIC_"))
        .cloned()
        .collect();
    for key in stale_keys {
        merged_env.remove(&key);
    }

    for (key, value) in &provider_env {
        // Skip empty template values
        if let Some(s) = value.as_str() {
            if !s.is_empty() {
                merged_env.insert(key.clone(), value.clone());
            }
        } else {
            merged_env.insert(key.clone(), value.clone());
        }
    }

    // Keep auth env mutually exclusive to avoid Claude auth conflict warnings.
    // If provider explicitly sets one auth style, drop the other stale key from previous providers.
    let has_auth_token = provider_env
        .get("ANTHROPIC_AUTH_TOKEN")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.is_empty());
    let has_api_key = provider_env
        .get("ANTHROPIC_API_KEY")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.is_empty());
    if has_auth_token {
        merged_env.remove("ANTHROPIC_API_KEY");
    } else if has_api_key {
        merged_env.remove("ANTHROPIC_AUTH_TOKEN");
    }

    // Update settings with merged env
    if let Some(ref mut settings_map) = settings.as_object_mut() {
        settings_map.insert("env".to_string(), Value::Object(merged_env));

        // Update top-level api_key and base_url from provider env
        if let Some(v) = provider_env.get("ANTHROPIC_BASE_URL") {
            settings_map.insert("base_url".to_string(), v.clone());
        }
        if let Some(v) = provider_env
            .get("ANTHROPIC_AUTH_TOKEN")
            .or_else(|| provider_env.get("ANTHROPIC_API_KEY"))
        {
            settings_map.insert("api_key".to_string(), v.clone());
        }
        if let Some(v) = provider_env.get("ANTHROPIC_MODEL") {
            settings_map.insert("model".to_string(), v.clone());
        }
    }

    write_live_settings_raw(&settings)?;
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

    #[test]
    fn apply_provider_to_live_restores_direct_settings_after_proxy_settings() {
        let _guard = env_lock().lock().expect("env lock");

        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let test_home = std::env::temp_dir().join(format!("cc-switch-live-test-{suffix}"));
        std::fs::create_dir_all(&test_home).expect("create test home");

        unsafe {
            std::env::set_var("CC_SWITCH_TEST_HOME", test_home.to_string_lossy().to_string());
        }

        let proxied = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "http://127.0.0.1:15721",
                "ANTHROPIC_AUTH_TOKEN": "PROXY_MANAGED",
                "ANTHROPIC_MODEL": "MiniMax-M2.7"
            }
        });
        apply_provider_to_live(&proxied).expect("apply proxied settings");

        let restored = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.minimaxi.com/v1",
                "ANTHROPIC_AUTH_TOKEN": "real-token",
                "ANTHROPIC_MODEL": "MiniMax-M2.7"
            }
        });
        apply_provider_to_live(&restored).expect("apply restored settings");

        let live = read_live_settings_raw().expect("read live settings");
        let env = live.get("env").and_then(Value::as_object).expect("env object");
        assert_eq!(
            live.get("base_url").and_then(Value::as_str),
            Some("https://api.minimaxi.com/v1")
        );
        assert_eq!(live.get("api_key").and_then(Value::as_str), Some("real-token"));
        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").and_then(Value::as_str),
            Some("https://api.minimaxi.com/v1")
        );
        assert_eq!(
            env.get("ANTHROPIC_AUTH_TOKEN").and_then(Value::as_str),
            Some("real-token")
        );

        unsafe {
            std::env::remove_var("CC_SWITCH_TEST_HOME");
        }
        let _ = std::fs::remove_dir_all(test_home);
    }

    #[test]
    fn apply_provider_to_live_restores_deepseek_api_key_after_proxy_settings() {
        let _guard = env_lock().lock().expect("env lock");

        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let test_home = std::env::temp_dir().join(format!("cc-switch-live-test-{suffix}"));
        std::fs::create_dir_all(&test_home).expect("create test home");

        unsafe {
            std::env::set_var("CC_SWITCH_TEST_HOME", test_home.to_string_lossy().to_string());
        }

        let proxied = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "http://127.0.0.1:15721",
                "ANTHROPIC_AUTH_TOKEN": "PROXY_MANAGED",
                "ANTHROPIC_MODEL": "deepseek-v4-pro"
            }
        });
        apply_provider_to_live(&proxied).expect("apply proxied settings");

        let restored = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
                "ANTHROPIC_API_KEY": "deepseek-real-key",
                "ANTHROPIC_MODEL": "deepseek-v4-pro"
            }
        });
        apply_provider_to_live(&restored).expect("apply restored settings");

        let live = read_live_settings_raw().expect("read live settings");
        let env = live.get("env").and_then(Value::as_object).expect("env object");
        assert_eq!(
            live.get("base_url").and_then(Value::as_str),
            Some("https://api.deepseek.com/anthropic")
        );
        assert_eq!(
            live.get("api_key").and_then(Value::as_str),
            Some("deepseek-real-key")
        );
        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").and_then(Value::as_str),
            Some("https://api.deepseek.com/anthropic")
        );
        assert_eq!(
            env.get("ANTHROPIC_API_KEY").and_then(Value::as_str),
            Some("deepseek-real-key")
        );
        assert_eq!(env.get("ANTHROPIC_AUTH_TOKEN"), None);

        unsafe {
            std::env::remove_var("CC_SWITCH_TEST_HOME");
        }
        let _ = std::fs::remove_dir_all(test_home);
    }

    #[test]
    fn apply_provider_to_live_replaces_stale_anthropic_keys_and_model() {
        let _guard = env_lock().lock().expect("env lock");

        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let test_home = std::env::temp_dir().join(format!("cc-switch-live-test-{suffix}"));
        std::fs::create_dir_all(&test_home).expect("create test home");
        unsafe {
            std::env::set_var("CC_SWITCH_TEST_HOME", test_home.to_string_lossy().to_string());
        }

        let initial = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://old.example.com",
                "ANTHROPIC_AUTH_TOKEN": "old-token",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "old-opus",
                "API_TIMEOUT_MS": "3000000"
            },
            "model": "old-model"
        });
        apply_provider_to_live(&initial).expect("apply initial");

        let refreshed = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
                "ANTHROPIC_API_KEY": "new-key",
                "ANTHROPIC_MODEL": "deepseek-v4-pro"
            }
        });
        apply_provider_to_live(&refreshed).expect("apply refreshed");

        let live = read_live_settings_raw().expect("read live settings");
        let env = live.get("env").and_then(Value::as_object).expect("env object");
        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").and_then(Value::as_str),
            Some("https://api.deepseek.com/anthropic")
        );
        assert_eq!(
            env.get("ANTHROPIC_API_KEY").and_then(Value::as_str),
            Some("new-key")
        );
        assert_eq!(env.get("ANTHROPIC_AUTH_TOKEN"), None);
        assert_eq!(env.get("ANTHROPIC_DEFAULT_OPUS_MODEL"), None);
        assert_eq!(env.get("API_TIMEOUT_MS").and_then(Value::as_str), Some("3000000"));
        assert_eq!(live.get("model").and_then(Value::as_str), Some("deepseek-v4-pro"));

        unsafe {
            std::env::remove_var("CC_SWITCH_TEST_HOME");
        }
        let _ = std::fs::remove_dir_all(test_home);
    }
}
