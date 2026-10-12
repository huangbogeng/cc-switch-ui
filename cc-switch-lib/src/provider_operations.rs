//! Durable coordination of a provider/live write and its database state.
use crate::{
    config,
    database::{Database, Provider},
    error::AppError,
    live,
    providers::AppType,
    settings,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, path::PathBuf};

const APP: &str = crate::DEFAULT_APP_TYPE;
#[derive(Clone, Serialize, Deserialize)]
struct Pending {
    path: PathBuf,
    database_path: PathBuf,
    before: Option<Vec<u8>>,
    after: Option<Vec<u8>>,
    publishing: bool,
    provider: Option<Provider>,
    #[serde(default)]
    backfill: Option<Provider>,
    #[serde(default)]
    selection: Option<String>,
    #[serde(default)]
    state_only: bool,
}

impl Pending {
    fn new(provider: Option<&Provider>) -> Self {
        Self {
            path: config::get_claude_settings_path(),
            database_path: config::get_app_config_dir().join("cc-switch.db"),
            before: None,
            after: None,
            publishing: false,
            provider: provider.cloned(),
            backfill: None,
            selection: None,
            state_only: false,
        }
    }
}
fn journal_path() -> PathBuf {
    config::get_home_dir().join(".cc-switch/provider-operation.json")
}
fn write_journal(pending: &Pending) -> Result<(), AppError> {
    let path = journal_path();
    if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(AppError::InvalidInput(
            "Refusing linked operation journal".into(),
        ));
    }
    config::write_json_file(&path, pending)
}
fn clear() -> Result<(), AppError> {
    let path = journal_path();
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::io(path, e)),
    }
}
fn commit(db: &Database, pending: &Pending) -> Result<(), AppError> {
    db.commit_provider_operation(
        APP,
        pending.provider.as_ref(),
        pending.backfill.as_ref(),
        pending.selection.as_deref(),
    )?;
    if let Some(id) = pending.selection.as_deref() {
        settings::set_current_provider(&AppType::ClaudeCode, Some(id))?;
    }
    clear()
}
fn digest(value: Option<&[u8]>) -> Option<String> {
    use sha2::{Digest, Sha256};
    value.map(|value| hex::encode(Sha256::digest(value)))
}

/// Complete only an operation whose publication was begun. External edits win.
/// A warning means completion preserved an externally edited live file.
pub fn recover(db: &Database) -> Result<Vec<String>, AppError> {
    let path = journal_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(AppError::InvalidInput(
            "Refusing linked operation journal".into(),
        ));
    }
    let pending: Pending = config::read_json_file(&path)?;
    // Fixed live path: a corrupted journal cannot redirect writes elsewhere.
    if (!pending.state_only && pending.path != config::get_claude_settings_path())
        || pending.database_path != config::get_app_config_dir().join("cc-switch.db")
    {
        return Err(AppError::Config("Pending operation has a different configuration path; restore its original configuration directory before retrying".into()));
    }
    if !pending.publishing {
        clear()?;
        return Ok(Vec::new());
    }
    if pending.state_only {
        commit(db, &pending)?;
        return Ok(Vec::new());
    }
    let mut warnings = Vec::new();
    config::update_json_transaction(
        &pending.path,
        |current| {
            let raw = match fs::read(&pending.path) {
                Ok(bytes) => Some(bytes),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(AppError::io(&pending.path, e)),
            };
            if digest(raw.as_deref()) == digest(pending.before.as_deref()) {
                if let Some(after) = &pending.after {
                    let value: Value = serde_json::from_slice(after)
                        .map_err(|e| AppError::json(&pending.path, e))?;
                    if !value.is_object() {
                        return Err(AppError::Config("Invalid staged live configuration".into()));
                    }
                    *current = value;
                }
            } else if digest(raw.as_deref()) != digest(pending.after.as_deref()) {
                warnings.push("An external edit to live config was preserved while completing the previous operation. Review the current configuration.".into());
            }
            Ok(())
        },
        |_, _| Ok(()),
        || Ok(()),
        || commit(db, &pending),
    )?;
    Ok(warnings)
}

pub fn apply(
    db: &Database,
    provider: Option<&Provider>,
    desired: &Value,
    base: &Value,
) -> Result<(), AppError> {
    run(db, Pending::new(provider), |current, _| {
        live::merge_provider_edit(current, desired, base)
    })
}

/// Publish a reviewed plan through the same durable write/database coordinator.
pub fn apply_edit_plan(
    db: &Database,
    provider: &Provider,
    observed: &Value,
    after: &Value,
) -> Result<(), AppError> {
    run(db, Pending::new(Some(provider)), |current, _| {
        if current != observed {
            return Err(AppError::Conflict {
                path:
                    "live file changed again before publication; retry to review the latest state"
                        .into(),
            });
        }
        *current = after.clone();
        Ok(())
    })
}

/// Save the outgoing live snapshot, apply the selected provider and persist both
/// selection stores. Returns whether live was applied. Caller serializes
/// provider/route mutations and recovers first.
pub fn switch(db: &Database, id: &str, port: u16, route_active: bool) -> Result<bool, AppError> {
    let provider = db
        .get_provider(id, APP)?
        .ok_or_else(|| AppError::InvalidInput("Provider not found".into()))?;
    if !route_active && db.get_live_backup(APP)?.is_some() {
        return Err(AppError::ProviderInUse(
            "Restore the interrupted local route before switching".into(),
        ));
    }
    if !provider.settings_config.is_object()
        || provider
            .settings_config
            .get("env")
            .is_some_and(|env| !env.is_object())
    {
        return Err(AppError::InvalidInput(
            "Selected provider configuration must be a JSON object with an object env".into(),
        ));
    }
    let current = settings::get_effective_current_provider(db, &AppType::ClaudeCode)?;
    let mut pending = Pending::new(None);
    pending.selection = Some(id.to_string());
    pending.state_only = route_active || current.as_deref() == Some(id);
    let old = current
        .as_deref()
        .filter(|old_id| *old_id != id)
        .map(|old_id| db.get_provider(old_id, APP))
        .transpose()?
        .flatten();
    let previous = old
        .as_ref()
        .and_then(|p| p.settings_config.get("env"))
        .and_then(Value::as_object);
    let desired = live::settings_for_live(&provider, port, false);
    let applied = !pending.state_only;
    run(db, pending, |current, pending| {
        pending.backfill = old.as_ref().map(|old| {
            let mut old = old.clone();
            old.settings_config = live::backfill_from_snapshot(&old.settings_config, current);
            old
        });
        live::merge_provider_settings(current, &desired, previous)
    })?;
    Ok(applied)
}

fn run(
    db: &Database,
    mut pending: Pending,
    mut patch: impl FnMut(&mut Value, &mut Pending) -> Result<(), AppError>,
) -> Result<(), AppError> {
    if recovery_pending() {
        return Err(AppError::Config(
            "An unfinished provider operation must be recovered before another write".into(),
        ));
    }
    // Selecting the Direct Config during takeover does not read or rewrite live.
    if pending.state_only {
        write_journal(&pending)?;
        pending.publishing = true;
        write_journal(&pending)?;
        return commit(db, &pending);
    }
    let path = pending.path.clone();
    let pending = std::cell::RefCell::new(pending);
    config::update_json_transaction(
        &path,
        |current| patch(current, &mut pending.borrow_mut()),
        |before, after| {
            let mut pending = pending.borrow_mut();
            pending.before = before.map(Vec::from);
            pending.after = after.map(Vec::from);
            pending.publishing = false;
            write_journal(&pending)
        },
        || {
            let mut pending = pending.borrow_mut();
            pending.publishing = true;
            write_journal(&pending)
        },
        || commit(db, &pending.borrow()),
    )
}

/// Discard recovery data only as part of an explicit destructive CLI clean.
/// Otherwise recovery must complete the operation before clearing the intent.
pub fn discard_for_clean() -> Result<(), AppError> {
    clear()
}

pub fn recovery_pending() -> bool {
    journal_path().exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::tests::{acquire_env_lock, cleanup, setup_test_home};
    use serde_json::json;

    #[test]
    fn reviewed_plan_rejects_a_later_live_change_before_writing_record_or_journal() {
        let _env = acquire_env_lock();
        let home = setup_test_home();
        let db = Database::init().unwrap();
        let provider: Provider = serde_json::from_value(
            json!({"id":"plan","name":"Plan","settingsConfig":{},"meta":{}}),
        )
        .unwrap();
        let path = config::get_claude_settings_path();
        let newer = json!({"env":{"API_TIMEOUT_MS":"new"}});
        config::write_json_file(&path, &newer).unwrap();
        assert!(apply_edit_plan(&db, &provider, &json!({}), &json!({"env":{}})).is_err());
        assert_eq!(config::read_json_file::<Value>(&path).unwrap(), newer);
        assert!(db.get_provider("plan", APP).unwrap().is_none());
        assert!(!recovery_pending());
        cleanup(home);
    }

    #[test]
    fn failed_database_commit_recovers_and_preserves_external_edits() {
        let _guard = acquire_env_lock();
        let home = setup_test_home();
        let db = Database::init().unwrap();
        let mut provider: Provider = serde_json::from_value(json!({"id":"edit-test", "name":"Test", "settingsConfig":{"env":{"ANTHROPIC_AUTH_TOKEN":"old"}}})).unwrap();
        db.save_provider(APP, &provider).unwrap();
        let base = provider.settings_config.clone();
        let path = config::get_claude_settings_path();
        config::write_json_file(&path, &json!({"env":{"ANTHROPIC_AUTH_TOKEN":"old", "API_TIMEOUT_MS":"123"}, "permissions":{"allow":["Read"]}})).unwrap();
        provider.settings_config["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("new");
        db.conn().execute_batch("CREATE TRIGGER fail_editor BEFORE UPDATE ON providers BEGIN SELECT RAISE(FAIL, 'injected failure'); END;").unwrap();
        assert!(apply(&db, Some(&provider), &provider.settings_config, &base).is_err());
        assert!(recovery_pending());
        let mut live: Value = config::read_json_file(&path).unwrap();
        assert_eq!(live["env"]["ANTHROPIC_AUTH_TOKEN"], "new");
        assert_eq!(
            db.get_provider(&provider.id, APP)
                .unwrap()
                .unwrap()
                .settings_config["env"]["ANTHROPIC_AUTH_TOKEN"],
            "old"
        );
        live["env"]["API_TIMEOUT_MS"] = json!("456");
        config::write_json_file(&path, &live).unwrap();
        db.conn()
            .execute_batch("DROP TRIGGER fail_editor;")
            .unwrap();
        assert_eq!(recover(&db).unwrap().len(), 1);
        assert_eq!(config::read_json_file::<Value>(&path).unwrap(), live);
        assert_eq!(
            db.get_provider(&provider.id, APP)
                .unwrap()
                .unwrap()
                .settings_config["env"]["ANTHROPIC_AUTH_TOKEN"],
            "new"
        );
        assert!(!recovery_pending());
        assert!(recover(&db).unwrap().is_empty());
        drop(db);
        cleanup(home);
    }

    #[test]
    fn prepared_intent_is_discarded_and_started_intent_is_replayed() {
        let _guard = acquire_env_lock();
        let home = setup_test_home();
        let db = Database::init().unwrap();
        let path = config::get_claude_settings_path();
        let before = json!({"env":{"ANTHROPIC_AUTH_TOKEN":"old"}});
        let after = json!({"env":{"ANTHROPIC_AUTH_TOKEN":"new"}});
        config::write_json_file(&path, &before).unwrap();
        let provider: Provider = serde_json::from_value(
            json!({"id":"intent-test", "name":"Test", "settingsConfig": after}),
        )
        .unwrap();
        let mut intent = Pending {
            path: path.clone(),
            database_path: config::get_app_config_dir().join("cc-switch.db"),
            before: Some(fs::read(&path).unwrap()),
            after: Some(serde_json::to_vec_pretty(&after).unwrap()),
            publishing: false,
            provider: Some(provider.clone()),
            backfill: None,
            selection: None,
            state_only: false,
        };
        write_journal(&intent).unwrap();
        recover(&db).unwrap();
        assert_eq!(config::read_json_file::<Value>(&path).unwrap(), before);
        assert!(db.get_provider(&provider.id, APP).unwrap().is_none());
        intent.publishing = true;
        // Older editor-save journals must remain recoverable after this upgrade.
        let mut legacy = serde_json::to_value(&intent).unwrap();
        for key in ["backfill", "selection", "state_only"] {
            legacy.as_object_mut().unwrap().remove(key);
        }
        config::write_json_file(&journal_path(), &legacy).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(journal_path()).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        recover(&db).unwrap();
        assert_eq!(config::read_json_file::<Value>(&path).unwrap(), after);
        assert!(db.get_provider(&provider.id, APP).unwrap().is_some());
        drop(db);
        cleanup(home);
    }
    fn selection_fixture() -> (Database, std::path::PathBuf) {
        let db = Database::init().unwrap();
        for id in ["selection-a", "selection-b"] {
            let provider: Provider = serde_json::from_value(json!({"id":id, "name":id, "settingsConfig":{"env":{"ANTHROPIC_AUTH_TOKEN":id, "ANTHROPIC_BASE_URL":format!("https://{id}.example")}}})).unwrap();
            db.save_provider(APP, &provider).unwrap();
        }
        settings::reload_settings().unwrap();
        db.set_current_provider("selection-a", APP).unwrap();
        settings::set_current_provider(&AppType::ClaudeCode, Some("selection-a")).unwrap();
        let path = config::get_claude_settings_path();
        config::write_json_file(&path, &json!({"env":{"ANTHROPIC_AUTH_TOKEN":"external-a", "API_TIMEOUT_MS":"123"}, "permissions":{"allow":["Read"]}})).unwrap();
        (db, path)
    }

    #[test]
    fn database_selection_failure_rolls_back_backfill_and_recovers_all_stores() {
        let _guard = acquire_env_lock();
        let home = setup_test_home();
        let (db, path) = selection_fixture();
        db.conn().execute_batch("CREATE TRIGGER fail_selection BEFORE UPDATE OF is_current ON providers WHEN NEW.is_current = 1 BEGIN SELECT RAISE(FAIL, 'injected selection failure'); END;").unwrap();
        assert!(switch(&db, "selection-b", 15721, false).is_err());
        assert!(recovery_pending());
        assert_eq!(
            db.get_current_provider_id(APP).unwrap().as_deref(),
            Some("selection-a")
        );
        assert_eq!(
            settings::get_current_provider(&AppType::ClaudeCode).as_deref(),
            Some("selection-a")
        );
        // Outgoing record and selection roll back together.
        assert_eq!(
            db.get_provider("selection-a", APP)
                .unwrap()
                .unwrap()
                .settings_config["env"]["ANTHROPIC_AUTH_TOKEN"],
            "selection-a"
        );
        let mut live: Value = config::read_json_file(&path).unwrap();
        assert_eq!(live["env"]["ANTHROPIC_AUTH_TOKEN"], "selection-b");
        assert_eq!(live["permissions"]["allow"][0], "Read");
        live["env"]["API_TIMEOUT_MS"] = json!("456");
        config::write_json_file(&path, &live).unwrap();
        db.conn()
            .execute_batch("DROP TRIGGER fail_selection;")
            .unwrap();
        // Simulate a process reload rather than trusting the previous memory value.
        settings::reload_settings().unwrap();
        assert_eq!(recover(&db).unwrap().len(), 1);
        assert_eq!(
            db.get_current_provider_id(APP).unwrap().as_deref(),
            Some("selection-b")
        );
        assert_eq!(
            settings::get_current_provider(&AppType::ClaudeCode).as_deref(),
            Some("selection-b")
        );
        assert_eq!(
            db.get_provider("selection-a", APP)
                .unwrap()
                .unwrap()
                .settings_config["env"]["ANTHROPIC_AUTH_TOKEN"],
            "external-a"
        );
        assert_eq!(config::read_json_file::<Value>(&path).unwrap(), live);
        assert!(!recovery_pending());
        settings::set_current_provider(&AppType::ClaudeCode, None).unwrap();
        drop(db);
        cleanup(home);
        settings::reload_settings().unwrap();
    }

    #[test]
    fn device_write_failure_keeps_memory_and_recovers_selection_without_live_rewrite() {
        let _guard = acquire_env_lock();
        let home = setup_test_home();
        let (db, path) = selection_fixture();
        let device = config::get_app_config_dir().join("settings.json");
        let retained = device.with_extension("retained");
        fs::rename(&device, &retained).unwrap();
        fs::create_dir(&device).unwrap();
        // A takeover-time selection must work independently of malformed external live data.
        fs::write(&path, "externally invalid JSON").unwrap();
        assert!(switch(&db, "selection-b", 15721, true).is_err());
        assert!(recovery_pending());
        assert_eq!(
            db.get_current_provider_id(APP).unwrap().as_deref(),
            Some("selection-b")
        );
        assert_eq!(
            settings::get_current_provider(&AppType::ClaudeCode).as_deref(),
            Some("selection-a")
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "externally invalid JSON"
        );
        fs::remove_dir(&device).unwrap();
        fs::rename(&retained, &device).unwrap();
        settings::reload_settings().unwrap();
        assert!(recover(&db).unwrap().is_empty());
        assert_eq!(
            settings::get_current_provider(&AppType::ClaudeCode).as_deref(),
            Some("selection-b")
        );
        let persisted: Value = config::read_json_file(&device).unwrap();
        assert_eq!(persisted["currentProviderClaudeCode"], "selection-b");
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "externally invalid JSON"
        );
        assert!(!recovery_pending());
        settings::set_current_provider(&AppType::ClaudeCode, None).unwrap();
        drop(db);
        cleanup(home);
        settings::reload_settings().unwrap();
    }

    #[test]
    fn invalid_live_and_unknown_target_leave_selection_and_records_untouched() {
        let _guard = acquire_env_lock();
        let home = setup_test_home();
        let (db, path) = selection_fixture();
        let old = db
            .get_provider("selection-a", APP)
            .unwrap()
            .unwrap()
            .settings_config;
        config::write_json_file(&path, &json!({"env": "invalid"})).unwrap();
        let before = fs::read(&path).unwrap();
        assert!(switch(&db, "selection-b", 15721, false).is_err());
        assert!(switch(&db, "missing", 15721, false).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(
            db.get_provider("selection-a", APP)
                .unwrap()
                .unwrap()
                .settings_config,
            old
        );
        assert_eq!(
            db.get_current_provider_id(APP).unwrap().as_deref(),
            Some("selection-a")
        );
        assert_eq!(
            settings::get_current_provider(&AppType::ClaudeCode).as_deref(),
            Some("selection-a")
        );
        assert!(!recovery_pending());
        settings::set_current_provider(&AppType::ClaudeCode, None).unwrap();
        drop(db);
        cleanup(home);
        settings::reload_settings().unwrap();
    }
    #[test]
    fn selecting_current_keeps_external_live_edits_and_clean_discards_replay() {
        let _guard = acquire_env_lock();
        let home = setup_test_home();
        let (db, path) = selection_fixture();
        let before = fs::read(&path).unwrap();
        switch(&db, "selection-a", 15721, false).unwrap();
        assert_eq!(fs::read(&path).unwrap(), before);
        assert!(!recovery_pending());
        db.conn().execute_batch("CREATE TRIGGER fail_selection BEFORE UPDATE OF is_current ON providers WHEN NEW.is_current = 1 BEGIN SELECT RAISE(FAIL, 'injected selection failure'); END;").unwrap();
        assert!(switch(&db, "selection-b", 15721, false).is_err());
        assert!(recovery_pending());
        discard_for_clean().unwrap();
        assert!(!recovery_pending());
        db.conn()
            .execute_batch("DROP TRIGGER fail_selection;")
            .unwrap();
        assert!(recover(&db).unwrap().is_empty());
        assert_eq!(
            db.get_current_provider_id(APP).unwrap().as_deref(),
            Some("selection-a")
        );
        settings::set_current_provider(&AppType::ClaudeCode, None).unwrap();
        drop(db);
        cleanup(home);
        settings::reload_settings().unwrap();
    }
}
