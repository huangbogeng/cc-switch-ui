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
        .map(|m| m.clone())
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

    // Update settings with merged env
    if let Some(ref mut settings_map) = settings.as_object_mut() {
        settings_map.insert("env".to_string(), Value::Object(merged_env));

        // Update top-level api_key and base_url from provider env
        if let Some(v) = provider_env.get("ANTHROPIC_BASE_URL") {
            settings_map.insert("base_url".to_string(), v.clone());
        }
        if let Some(v) = provider_env.get("ANTHROPIC_AUTH_TOKEN") {
            settings_map.insert("api_key".to_string(), v.clone());
        }
    }

    write_live_settings_raw(&settings)?;
    log::info!(
        "Applied provider settings to live Claude config at {:?}",
        get_live_settings_path()
    );
    Ok(())
}
