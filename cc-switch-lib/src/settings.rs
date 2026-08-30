//! Device-level settings
//!
//! Manages a local `settings.json` file stored in the app config directory
//! (`~/.cc-switch/settings.json`). Settings are device-local (not synced)
//! and store preferences such as the current provider ID.
//!
//! Architecture mirrors the original cc-switch: a global `OnceLock<RwLock<AppSettings>>`
//! loaded once at startup and persisted on mutation.

use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::OnceLock;
use std::sync::RwLock;

use crate::error::AppError;
use crate::providers::AppType;

// ============================================================================
// AppSettings
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// Device-level current provider ID for Claude Code.
    #[serde(default)]
    pub current_provider_claude_code: Option<String>,

    /// Device-level current provider ID for Codex (reserved).
    #[serde(default)]
    pub current_provider_codex: Option<String>,

    /// Device-level current provider ID for OpenCode (reserved).
    #[serde(default)]
    pub current_provider_opencode: Option<String>,
}

impl AppSettings {
    fn settings_path() -> Option<std::path::PathBuf> {
        let dir = crate::config::get_app_config_dir();
        Some(dir.join("settings.json"))
    }

    fn load_from_file() -> Self {
        let path = match Self::settings_path() {
            Some(p) => p,
            None => return Self::default(),
        };

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(settings) => settings,
                Err(err) => {
                    log::warn!(
                        "Failed to parse settings file ({}), using defaults: {err}",
                        path.display()
                    );
                    Self::default()
                }
            },
            Err(err) => {
                log::warn!(
                    "Failed to read settings file ({}), using defaults: {err}",
                    path.display()
                );
                Self::default()
            }
        }
    }

    fn save(&self) -> Result<(), AppError> {
        let path = Self::settings_path()
            .ok_or_else(|| AppError::Config("Cannot determine settings path".to_string()))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| AppError::JsonSerialize { source: e })?;
        crate::config::atomic_write(&path, json.as_bytes())
    }
}

// ============================================================================
// Global store
// ============================================================================

static SETTINGS_STORE: OnceLock<RwLock<AppSettings>> = OnceLock::new();

fn settings_store() -> &'static RwLock<AppSettings> {
    SETTINGS_STORE.get_or_init(|| RwLock::new(AppSettings::load_from_file()))
}

/// Reload settings from disk (used after import/migration).
pub fn reload_settings() -> Result<(), AppError> {
    let fresh = AppSettings::load_from_file();
    let mut store = settings_store()
        .write()
        .map_err(|e| AppError::Config(format!("Settings store poisoned: {e}")))?;
    *store = fresh;
    Ok(())
}

// ============================================================================
// Public API
// ============================================================================

/// Get Claude override directory — returns None for headless server
pub fn get_claude_override_dir() -> Option<std::path::PathBuf> {
    None
}

/// Device-level current provider ID for an app type (no existence check).
pub fn get_current_provider(app_type: &AppType) -> Option<String> {
    let store = settings_store().read().ok()?;
    match app_type {
        AppType::ClaudeCode => store.current_provider_claude_code.clone(),
        AppType::Codex => store.current_provider_codex.clone(),
        AppType::OpenCode => store.current_provider_opencode.clone(),
    }
}

/// Set the device-level current provider ID for an app type.
pub fn set_current_provider(app_type: &AppType, id: Option<&str>) -> Result<(), AppError> {
    let id_owned = id.map(|s| s.to_string());
    let mut store = settings_store()
        .write()
        .map_err(|e| AppError::Config(format!("Settings store poisoned: {e}")))?;

    match app_type {
        AppType::ClaudeCode => store.current_provider_claude_code = id_owned,
        AppType::Codex => store.current_provider_codex = id_owned,
        AppType::OpenCode => store.current_provider_opencode = id_owned,
    }

    store.save()
}

/// Get the effective current provider ID — verifies existence in DB with fallback.
///
/// Priority:
/// 1. Device-level setting (if the provider exists in the database)
/// 2. Database `is_current` flag (fallback for new devices)
pub fn get_effective_current_provider(
    db: &crate::database::Database,
    app_type: &AppType,
) -> Result<Option<String>, AppError> {
    // Try device-level setting first
    if let Some(local_id) = get_current_provider(app_type) {
        let providers = db.list_providers(app_type.as_str())?;
        if providers.contains_key(&local_id) {
            return Ok(Some(local_id));
        }

        // Not found — clear stale local setting
        log::warn!(
            "Local setting provider '{}' not in database for '{}', clearing",
            local_id,
            app_type.as_str()
        );
        let _ = set_current_provider(app_type, None);
    }

    // Fall back to database is_current
    db.get_current_provider_id(app_type.as_str())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_roundtrip() {
        let settings = AppSettings::default();
        let json = serde_json::to_string(&settings).expect("serialize");
        let deserialized: AppSettings = serde_json::from_str(&json).expect("deserialize");
        assert!(deserialized.current_provider_claude_code.is_none());
    }
}
