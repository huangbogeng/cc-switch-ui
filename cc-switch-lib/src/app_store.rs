//! App store module - STUB for Phase 1
//!
//! Provides app config directory override - not used in headless server.

use std::path::PathBuf;

/// Get cached app config dir override - returns None for headless server
pub fn get_app_config_dir_override() -> Option<PathBuf> {
    None
}
