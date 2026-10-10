//! Live config synchronization module
//!
//! Handles syncing provider settings to Claude Code's live config files.
//!
//! Writes follow a key-ownership model (see [`floor`]): key fields
//! (`ANTHROPIC_*`, `AWS_*`, protocol selectors, ...) belong to the provider and
//! are cleared and rewritten on every switch; provider-exclusive fields
//! (compat switches, window values) are removed on switch-out only when the
//! user never edited them; everything else is a user key and is preserved
//! untouched forever.

#[cfg(test)]
use crate::config::write_json_file;
use crate::config::{get_claude_settings_path, read_json_file, update_json_file};
use crate::error::AppError;
use serde_json::{Map, Value};
use std::path::PathBuf;

pub mod floor;

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
        return Ok(Value::Object(Map::new()));
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

/// Capture the provider-owned subset of the current live config.
///
/// Returns `{ "env": { owned keys only } }` — key fields and exclusive fields
/// currently present in the live file. User keys are deliberately excluded:
/// they never travel with providers, they stay in the live file permanently.
///
/// Used for the first-launch import of an existing Claude Code installation.
pub fn capture_owned_live_config() -> Result<Value, AppError> {
    let live = read_live_settings_raw()?;
    let mut owned = Map::new();
    if let Some(env) = live.get("env").and_then(Value::as_object) {
        for (key, value) in env {
            if floor::claude_owned_env(key) {
                owned.insert(key.clone(), value.clone());
            }
        }
    }
    Ok(Value::Object(Map::from_iter([(
        "env".to_string(),
        Value::Object(owned),
    )])))
}

/// Read current live settings and capture the owned keys into a provider record.
///
/// This captures the user's current live config (which may include direct edits
/// such as a `/model` selection) and merges the owned subset into the provider's
/// existing `settings_config`. Without this backfill, edits made directly in the
/// live file would be lost on the next switch.
///
/// When the live file contains no owned keys at all, the existing config is
/// returned unchanged — an empty live file must not wipe a provider record.
pub fn backfill_current_live_config(existing: &Value) -> Result<Value, AppError> {
    let captured = capture_owned_live_config()?;
    let captured_env = captured
        .get("env")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if captured_env.is_empty() {
        return Ok(existing.clone());
    }
    let mut result = existing.clone();
    if !result.is_object() {
        result = Value::Object(Map::new());
    }
    result
        .as_object_mut()
        .expect("normalized above")
        .insert("env".to_string(), Value::Object(captured_env));
    Ok(result)
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

/// Auto-mode server-side classifier is only supported on the official
/// endpoint. The local proxy is a gateway from Claude Code's perspective, so
/// takeover always disables it to avoid blocking prompts.
pub const CLAUDE_TAKEOVER_AUTO_MODE_SERVER: &str = "0";

/// Build the sanitized live-settings payload for a provider.
///
/// When `use_proxy` is true the key fields are rewritten to route all Claude
/// Code requests through the local proxy; the provider's own compat switches
/// and other non-key fields are kept because the proxy still forwards to the
/// same provider. Otherwise the provider's settings_config env is used as-is.
///
/// The returned payload is the *desired* env; [`apply_provider_to_live`] merges
/// it into the live file under the ownership rules.
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
        // Drop the provider's connection/auth keys — the real credentials stay
        // in the database and the proxy injects them during forwarding.
        env.retain(|key, _| !floor::claude_floor_env(key));
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
        env.insert(
            "CLAUDE_CODE_AUTO_MODE_SERVER".to_string(),
            serde_json::json!(CLAUDE_TAKEOVER_AUTO_MODE_SERVER),
        );
    }

    settings
}

/// Write Claude settings to disk (atomic write)
#[cfg(test)]
fn write_live_settings_raw(settings: &Value) -> Result<(), AppError> {
    let path = get_claude_settings_path();
    write_json_file(&path, settings)
}

/// Apply provider settings to live Claude config under the ownership model.
///
/// The provider record is first *projected* down to the keys it owns (key
/// fields and exclusive fields). Any other key in the record — a user key such
/// as `API_TIMEOUT_MS` that an older version backfilled, or that the editor
/// still carries — is ignored, so a stale provider value can never clobber a
/// live edit the user made.
///
/// Reads the current `~/.claude/settings.json` and rebuilds its `env`:
/// - owned keys present in the desired env are written (in place, preserving
///   order);
/// - key fields absent from the desired env are removed (they belonged to the
///   previous provider);
/// - exclusive fields absent from the desired env are removed only when
///   `previous_env` shows the previous provider wrote the exact same value
///   (i.e. the user never edited them);
/// - every other live key is a user key and is preserved untouched.
///
/// Non-env top-level fields of the live file are always preserved.
///
/// `previous_env` is the env of the provider record that last wrote the live
/// file, captured *before* any backfill. Pass `None` when unknown.
pub fn apply_provider_to_live(
    settings_config: &Value,
    previous_env: Option<&Map<String, Value>>,
) -> Result<(), AppError> {
    let desired = sanitize_claude_settings_for_live(settings_config);
    let desired_env = desired
        .get("env")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    // Project the record onto the keys the provider owns; user keys it may
    // still carry are dropped here so they never reach the live file.
    let desired_owned: Map<String, Value> = desired_env
        .into_iter()
        .filter(|(key, _)| floor::claude_owned_env(key))
        .collect();

    update_json_file(&get_claude_settings_path(), |current| {
        let current_obj = current.as_object_mut().ok_or_else(|| {
            AppError::Config(format!(
                "live settings at {:?} is not a JSON object; refusing to merge",
                get_claude_settings_path()
            ))
        })?;

        let mut merged = Map::new();
        if let Some(live_env) = current_obj.get("env").and_then(Value::as_object) {
            for (key, value) in live_env {
                if let Some(new_value) = desired_owned.get(key) {
                    merged.insert(key.clone(), new_value.clone());
                    continue;
                }
                if floor::claude_floor_env(key) {
                    continue;
                }
                if floor::claude_exclusive_env(key)
                    && previous_env.is_some_and(|prev| prev.get(key) == Some(value))
                {
                    continue;
                }
                merged.insert(key.clone(), value.clone());
            }
        }
        for (key, value) in &desired_owned {
            merged.entry(key.clone()).or_insert_with(|| value.clone());
        }

        current_obj.insert("env".into(), Value::Object(merged));
        Ok(())
    })?;
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

    fn env_of(value: &Value) -> &Map<String, Value> {
        value.get("env").and_then(Value::as_object).unwrap()
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
        apply_provider_to_live(&settings, None).expect("apply settings");

        let live = read_live_settings_raw().expect("read live settings");
        // No top-level fields extracted from env
        assert_eq!(live.get("base_url"), None);
        assert_eq!(live.get("api_key"), None);
        // Env content matches input
        assert_eq!(
            live.pointer("/env/ANTHROPIC_BASE_URL")
                .and_then(Value::as_str),
            Some("https://api.deepseek.com/anthropic")
        );
        assert_eq!(
            live.pointer("/env/ANTHROPIC_API_KEY")
                .and_then(Value::as_str),
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
        apply_provider_to_live(&settings, None).expect("apply settings");

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
            live.pointer("/env/ANTHROPIC_BASE_URL")
                .and_then(Value::as_str),
            Some("https://api.deepseek.com/anthropic")
        );

        cleanup(test_home);
    }

    #[test]
    fn apply_provider_to_live_replaces_previous_key_fields() {
        let _guard = acquire_env_lock();
        let test_home = setup_test_home();

        // Pre-write a file with non-env root fields that must survive
        let prewrite = json!({
            "project": "my-project",
            "theme": "dark"
        });
        write_live_settings_raw(&prewrite).expect("pre-write existing");

        // First write: proxy takeover style env
        let first = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "http://127.0.0.1:15721",
                "ANTHROPIC_AUTH_TOKEN": "PROXY_MANAGED"
            }
        });
        apply_provider_to_live(&first, None).expect("apply first");

        // Second write: direct provider — previous key fields must be gone
        let second = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
                "ANTHROPIC_API_KEY": "real-key"
            }
        });
        apply_provider_to_live(&second, None).expect("apply second");

        let live = read_live_settings_raw().expect("read live settings");
        // project and theme from pre-write must survive
        assert_eq!(
            live.get("project").and_then(Value::as_str),
            Some("my-project")
        );
        assert_eq!(live.get("theme").and_then(Value::as_str), Some("dark"));
        assert_eq!(
            live.pointer("/env/ANTHROPIC_BASE_URL")
                .and_then(Value::as_str),
            Some("https://api.deepseek.com/anthropic")
        );
        assert_eq!(
            live.pointer("/env/ANTHROPIC_API_KEY")
                .and_then(Value::as_str),
            Some("real-key")
        );
        // PROXY_MANAGED key field from first write is gone
        assert_eq!(live.pointer("/env/ANTHROPIC_AUTH_TOKEN"), None);

        cleanup(test_home);
    }

    #[test]
    fn user_env_keys_survive_every_switch() {
        let _guard = acquire_env_lock();
        let test_home = setup_test_home();

        let user_keys = json!({
            "env": {
                "DISABLE_TELEMETRY": "1",
                "API_TIMEOUT_MS": "60000",
                "CLAUDE_CODE_USE_POWERSHELL_TOOL": "1"
            }
        });
        write_live_settings_raw(&user_keys).expect("pre-write user keys");

        // Proxy takeover must not wipe user keys
        let takeover = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "http://127.0.0.1:15721",
                "ANTHROPIC_AUTH_TOKEN": "PROXY_MANAGED"
            }
        });
        apply_provider_to_live(&takeover, None).expect("takeover");
        let live = read_live_settings_raw().expect("read");
        assert_eq!(
            live.pointer("/env/DISABLE_TELEMETRY")
                .and_then(Value::as_str),
            Some("1")
        );
        assert_eq!(
            live.pointer("/env/API_TIMEOUT_MS").and_then(Value::as_str),
            Some("60000")
        );

        // Direct switch must not wipe them either
        let direct = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
                "ANTHROPIC_AUTH_TOKEN": "sk-x"
            }
        });
        apply_provider_to_live(&direct, None).expect("direct");
        let live = read_live_settings_raw().expect("read");
        assert_eq!(
            live.pointer("/env/DISABLE_TELEMETRY")
                .and_then(Value::as_str),
            Some("1")
        );
        assert_eq!(
            live.pointer("/env/CLAUDE_CODE_USE_POWERSHELL_TOOL")
                .and_then(Value::as_str),
            Some("1")
        );

        cleanup(test_home);
    }

    #[test]
    fn provider_record_cannot_clobber_live_user_key() {
        let _guard = acquire_env_lock();
        let test_home = setup_test_home();

        // The user tuned API_TIMEOUT_MS directly in the live file.
        let live = json!({
            "env": {
                "DISABLE_TELEMETRY": "1",
                "API_TIMEOUT_MS": "120000"
            }
        });
        write_live_settings_raw(&live).expect("pre-write live");

        // The provider record still carries a stale API_TIMEOUT_MS (backfilled
        // by an older version) plus its own connection keys.
        let provider = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
                "ANTHROPIC_AUTH_TOKEN": "sk-x",
                "API_TIMEOUT_MS": "3000000",
                "DISABLE_TELEMETRY": "0"
            }
        });
        apply_provider_to_live(&provider, None).expect("apply provider");

        let live = read_live_settings_raw().expect("read");
        // Owned keys are written from the provider.
        assert_eq!(
            live.pointer("/env/ANTHROPIC_BASE_URL")
                .and_then(Value::as_str),
            Some("https://api.deepseek.com/anthropic")
        );
        // User keys keep their live value — the stale record value is ignored.
        assert_eq!(
            live.pointer("/env/API_TIMEOUT_MS").and_then(Value::as_str),
            Some("120000")
        );
        assert_eq!(
            live.pointer("/env/DISABLE_TELEMETRY")
                .and_then(Value::as_str),
            Some("1")
        );

        cleanup(test_home);
    }

    #[test]
    fn exclusive_fields_leave_only_when_unchanged() {
        let _guard = acquire_env_lock();
        let test_home = setup_test_home();

        // Gateway provider wrote AUTO_MODE=0 and DISABLE_ARTIFACT=1
        let gateway = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://gw.example",
                "ANTHROPIC_AUTH_TOKEN": "sk-gw",
                "CLAUDE_CODE_AUTO_MODE_SERVER": "0",
                "CLAUDE_CODE_DISABLE_ARTIFACT": "1"
            }
        });
        apply_provider_to_live(&gateway, None).expect("apply gateway");

        // User edits one exclusive key in the live file
        let mut live = read_live_settings_raw().expect("read");
        live.pointer_mut("/env/CLAUDE_CODE_DISABLE_ARTIFACT")
            .unwrap()
            .clone_from(&json!("0"));
        write_live_settings_raw(&live).expect("write user edit");

        // Switch to official provider that writes no exclusive keys.
        // previous_env is the gateway record captured before backfill.
        let previous = env_of(&gateway).clone();
        let official = json!({
            "env": { "ANTHROPIC_AUTH_TOKEN": "sk-official" }
        });
        apply_provider_to_live(&official, Some(&previous)).expect("apply official");

        let live = read_live_settings_raw().expect("read");
        // Unchanged since the gateway wrote it → removed
        assert_eq!(live.pointer("/env/CLAUDE_CODE_AUTO_MODE_SERVER"), None);
        // User edited it → preserved
        assert_eq!(
            live.pointer("/env/CLAUDE_CODE_DISABLE_ARTIFACT")
                .and_then(Value::as_str),
            Some("0")
        );
        // Gateway key fields are gone
        assert_eq!(live.pointer("/env/ANTHROPIC_BASE_URL"), None);

        cleanup(test_home);
    }

    #[test]
    fn backfill_captures_owned_keys_only() {
        let _guard = acquire_env_lock();
        let test_home = setup_test_home();

        let live = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.minimaxi.com/v1",
                "ANTHROPIC_MODEL": "user-picked-model",
                "CLAUDE_CODE_AUTO_MODE_SERVER": "0",
                "DISABLE_TELEMETRY": "1"
            },
            "hooks": { "Stop": [] }
        });
        write_live_settings_raw(&live).expect("write live");

        let existing = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.minimaxi.com/v1",
                "ANTHROPIC_MODEL": "MiniMax-M2.7",
                "API_TIMEOUT_MS": "3000000"
            },
            "notes": "keep me"
        });
        let backfilled = backfill_current_live_config(&existing).expect("backfill");

        let env = env_of(&backfilled);
        // User edit captured into the record
        assert_eq!(
            env.get("ANTHROPIC_MODEL").and_then(Value::as_str),
            Some("user-picked-model")
        );
        // Exclusive key captured
        assert_eq!(
            env.get("CLAUDE_CODE_AUTO_MODE_SERVER")
                .and_then(Value::as_str),
            Some("0")
        );
        // User key never travels with providers
        assert_eq!(env.get("DISABLE_TELEMETRY"), None);
        // Record key absent from live is dropped (live is truth)...
        assert_eq!(env.get("API_TIMEOUT_MS"), None);
        // ...but non-env record fields survive
        assert_eq!(
            backfilled.get("notes").and_then(Value::as_str),
            Some("keep me")
        );
        // Top-level live fields are not captured into the record
        assert_eq!(backfilled.get("hooks"), None);

        cleanup(test_home);
    }

    #[test]
    fn backfill_keeps_existing_when_live_has_no_owned_keys() {
        let _guard = acquire_env_lock();
        let test_home = setup_test_home();

        let live = json!({ "env": { "DISABLE_TELEMETRY": "1" } });
        write_live_settings_raw(&live).expect("write live");

        let existing = json!({ "env": { "ANTHROPIC_BASE_URL": "https://x" } });
        let backfilled = backfill_current_live_config(&existing).expect("backfill");
        assert_eq!(backfilled, existing);

        cleanup(test_home);
    }

    fn provider_with_env(env: Value) -> crate::database::Provider {
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
    fn proxy_mode_writes_takeover_key_fields_and_keeps_compat_switches() {
        let provider = provider_with_env(json!({
            "ANTHROPIC_BASE_URL": "https://api.minimaxi.com/v1",
            "ANTHROPIC_AUTH_TOKEN": "real-token",
            "API_TIMEOUT_MS": "3000000",
            "CLAUDE_CODE_DISABLE_ARTIFACT": "1"
        }));

        let settings = settings_for_live(&provider, 15721, true);
        let env = env_of(&settings);

        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").and_then(Value::as_str),
            Some("http://127.0.0.1:15721")
        );
        assert_eq!(
            env.get("ANTHROPIC_AUTH_TOKEN").and_then(Value::as_str),
            Some("PROXY_MANAGED")
        );
        assert_eq!(
            env.get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
                .and_then(Value::as_str),
            Some("claude-haiku-4-5")
        );
        assert_eq!(
            env.get("ANTHROPIC_DEFAULT_SONNET_MODEL")
                .and_then(Value::as_str),
            Some("claude-sonnet-4-6")
        );
        assert_eq!(
            env.get("ANTHROPIC_DEFAULT_OPUS_MODEL")
                .and_then(Value::as_str),
            Some("claude-opus-4-8")
        );
        // The proxy is a gateway too — auto-mode classifier must stay off
        assert_eq!(
            env.get("CLAUDE_CODE_AUTO_MODE_SERVER")
                .and_then(Value::as_str),
            Some("0")
        );
        // Real credentials must not leak into the takeover payload
        assert!(!env.values().any(|v| v.as_str() == Some("real-token")));
        // Provider compat switches and timeouts survive takeover
        assert_eq!(
            env.get("CLAUDE_CODE_DISABLE_ARTIFACT")
                .and_then(Value::as_str),
            Some("1")
        );
        assert_eq!(
            env.get("API_TIMEOUT_MS").and_then(Value::as_str),
            Some("3000000")
        );
    }

    #[test]
    fn restore_mode_preserves_direct_provider_settings() {
        let provider = provider_with_env(json!({
            "ANTHROPIC_BASE_URL": "https://api.minimaxi.com/v1",
            "ANTHROPIC_AUTH_TOKEN": "real-token"
        }));

        let settings = settings_for_live(&provider, 15721, false);
        let env = env_of(&settings);

        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").and_then(Value::as_str),
            Some("https://api.minimaxi.com/v1")
        );
        assert_eq!(
            env.get("ANTHROPIC_AUTH_TOKEN").and_then(Value::as_str),
            Some("real-token")
        );
        // Direct mode adds no takeover keys
        assert_eq!(env.get("CLAUDE_CODE_AUTO_MODE_SERVER"), None);
    }

    #[test]
    fn stopping_after_route_change_cleans_the_original_takeover_fields() {
        let _guard = acquire_env_lock();
        let test_home = setup_test_home();
        let kimi = provider_with_env(json!({
            "ANTHROPIC_BASE_URL": "https://api.kimi.com/coding/",
            "ANTHROPIC_AUTH_TOKEN": "kimi-key",
            "CLAUDE_CODE_MAX_CONTEXT_TOKENS": "262144",
            "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "262144",
            "CLAUDE_CODE_DISABLE_ARTIFACT": "1"
        }));
        let takeover = settings_for_live(&kimi, 15721, true);
        apply_provider_to_live(&takeover, None).unwrap();
        let snapshot = env_of(&takeover).clone();
        // A user edit must survive, while unchanged Kimi fields must be removed.
        let mut current = read_live_settings_raw().unwrap();
        current["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"] = json!("0");
        write_live_settings_raw(&current).unwrap();
        let new_target = provider_with_env(json!({
            "ANTHROPIC_BASE_URL": "https://example.com",
            "ANTHROPIC_AUTH_TOKEN": "new-key"
        }));
        apply_provider_to_live(
            &settings_for_live(&new_target, 15721, false),
            Some(&snapshot),
        )
        .unwrap();
        let restored = read_live_settings_raw().unwrap();
        assert_eq!(
            restored.pointer("/env/CLAUDE_CODE_MAX_CONTEXT_TOKENS"),
            None
        );
        assert_eq!(
            restored.pointer("/env/CLAUDE_CODE_AUTO_COMPACT_WINDOW"),
            None
        );
        assert_eq!(restored.pointer("/env/CLAUDE_CODE_AUTO_MODE_SERVER"), None);
        assert_eq!(restored["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"], "0");
        assert_eq!(restored["env"]["ANTHROPIC_AUTH_TOKEN"], "new-key");
        cleanup(test_home);
    }

    #[test]
    fn apply_refuses_non_object_live_file() {
        let _guard = acquire_env_lock();
        let test_home = setup_test_home();

        let path = get_claude_settings_path();
        std::fs::create_dir_all(path.parent().unwrap()).expect("create dir");
        std::fs::write(&path, "[1,2,3]").expect("write array");

        let settings = json!({ "env": { "ANTHROPIC_BASE_URL": "https://x" } });
        let err = apply_provider_to_live(&settings, None).expect_err("must refuse");
        assert!(matches!(err, AppError::Config(_)));
        // File untouched
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[1,2,3]");

        cleanup(test_home);
    }
}
