//! Durable coordination of a provider/live write and its database state.
use crate::{
    config,
    database::{Database, Provider},
    error::AppError,
    live,
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
    if let Some(provider) = &pending.provider {
        db.save_provider(APP, provider)?;
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
    if pending.path != config::get_claude_settings_path()
        || pending.database_path != config::get_app_config_dir().join("cc-switch.db")
    {
        return Err(AppError::Config("Pending operation has a different configuration path; restore its original configuration directory before retrying".into()));
    }
    if !pending.publishing {
        clear()?;
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
    if journal_path().exists() {
        return Err(AppError::Config(
            "An unfinished provider operation must be recovered before another write".into(),
        ));
    }
    let path = config::get_claude_settings_path();
    let pending = std::cell::RefCell::new(Pending {
        path: path.clone(),
        database_path: config::get_app_config_dir().join("cc-switch.db"),
        before: None,
        after: None,
        publishing: false,
        provider: provider.cloned(),
    });
    config::update_json_transaction(
        &path,
        |current| live::merge_provider_edit(current, desired, base),
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

pub fn recovery_pending() -> bool {
    journal_path().exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::tests::{acquire_env_lock, cleanup, setup_test_home};
    use serde_json::json;

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
        };
        write_journal(&intent).unwrap();
        recover(&db).unwrap();
        assert_eq!(config::read_json_file::<Value>(&path).unwrap(), before);
        assert!(db.get_provider(&provider.id, APP).unwrap().is_none());
        intent.publishing = true;
        write_journal(&intent).unwrap();
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
}
