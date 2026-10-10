//! Edit user-owned Claude env fields without changing provider connection fields.
use crate::config::{read_first_write_backup, update_json_file};
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

const TIMEOUT: &str = "API_TIMEOUT_MS";
const TRAFFIC: &str = "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserValues {
    #[serde(deserialize_with = "required_optional")]
    pub timeout_ms: Option<String>,
    #[serde(deserialize_with = "required_optional")]
    pub nonessential_traffic: Option<String>,
}

fn required_optional<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSettings {
    pub path: String,
    pub revision: String,
    pub values: UserValues,
    pub initial_values: Option<UserValues>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveRequest {
    pub revision: String,
    pub values: UserValues,
}

pub fn read(path: &Path) -> Result<UserSettings, AppError> {
    let value = match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| AppError::json(path, e))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(e) => return Err(AppError::io(path, e)),
    };
    let values = values_of(&value)?;
    Ok(UserSettings {
        path: path.display().to_string(),
        revision: revision(&values),
        values,
        initial_values: read_first_write_backup(path)?
            .as_ref()
            .map(values_of)
            .transpose()?,
    })
}

fn values_of(value: &Value) -> Result<UserValues, AppError> {
    if !value.is_object() {
        return Err(AppError::Config(
            "Configuration root must be an object".into(),
        ));
    }
    let env = match value.get("env") {
        None => None,
        Some(value) => Some(
            value
                .as_object()
                .ok_or_else(|| AppError::Config("Configuration env must be an object".into()))?,
        ),
    };
    let get = |key: &str| -> Result<Option<String>, AppError> {
        env.and_then(|env| env.get(key))
            .map(|value| {
                value.as_str().map(str::to_string).ok_or_else(|| {
                    AppError::Config(format!(
                        "{key} must be a string; edit the live file to correct it"
                    ))
                })
            })
            .transpose()
    };
    Ok(UserValues {
        timeout_ms: get(TIMEOUT)?,
        nonessential_traffic: get(TRAFFIC)?,
    })
}

fn revision(values: &UserValues) -> String {
    // Hash only these two fields. Provider switches and unrelated edits may proceed.
    hex::encode(Sha256::digest(
        serde_json::to_vec(values).expect("string values serialize"),
    ))
}

pub fn save(path: &Path, request: &SaveRequest) -> Result<(), AppError> {
    patch(path, &request.revision, &request.values, true)
}

pub fn restore(path: &Path, expected_revision: &str) -> Result<(), AppError> {
    let original = read_first_write_backup(path)?
        .ok_or_else(|| AppError::InvalidInput("No first-write backup is available".into()))?;
    // Restore only this panel's two user fields, not the full snapshot or credentials.
    patch(path, expected_revision, &values_of(&original)?, false)
}

fn patch(
    path: &Path,
    expected_revision: &str,
    desired: &UserValues,
    validate: bool,
) -> Result<(), AppError> {
    update_json_file(path, |current| {
        let old = values_of(current)?;
        if revision(&old) != expected_revision {
            return Err(AppError::Conflict {
                path: path.display().to_string(),
            });
        }
        if validate && desired.timeout_ms != old.timeout_ms {
            if let Some(timeout) = &desired.timeout_ms {
                if timeout.is_empty()
                    || !timeout.bytes().all(|c| c.is_ascii_digit())
                    || timeout.parse::<u64>().ok().is_none_or(|value| value == 0)
                {
                    return Err(AppError::InvalidInput(
                        "API timeout must be a positive integer in milliseconds, or unset".into(),
                    ));
                }
            }
        }
        if validate
            && desired.nonessential_traffic != old.nonessential_traffic
            && desired
                .nonessential_traffic
                .as_deref()
                .is_some_and(|value| !matches!(value, "0" | "1"))
        {
            return Err(AppError::InvalidInput(
                "Nonessential traffic must be unset, 0 or 1".into(),
            ));
        }
        if *desired == old {
            return Ok(());
        }
        if current.get("env").is_none() {
            current["env"] = json!({});
        }
        let env = current["env"].as_object_mut().expect("validated env");
        for (key, value) in [
            (TIMEOUT, &desired.timeout_ms),
            (TRAFFIC, &desired.nonessential_traffic),
        ] {
            match value {
                Some(value) => {
                    env.insert(key.into(), json!(value));
                }
                None => {
                    env.remove(key);
                }
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::tests::{acquire_env_lock, cleanup, setup_test_home};

    #[test]
    fn user_edits_rebase_unrelated_changes_and_restore_only_user_fields() {
        let _env = acquire_env_lock();
        let home = setup_test_home();
        let path = crate::config::get_claude_settings_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = json!({"env": {TIMEOUT: "2000", TRAFFIC: "1", "ANTHROPIC_AUTH_TOKEN": "original"}, "permissions": {"allow": ["Read"]}});
        std::fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
        let loaded = read(&path).unwrap();
        let request = SaveRequest {
            revision: loaded.revision,
            values: UserValues {
                timeout_ms: Some("4000".into()),
                nonessential_traffic: Some("0".into()),
            },
        };
        crate::config::update_json_file(&path, |value| {
            value["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("new");
            value["external"] = json!(42);
            Ok(())
        })
        .unwrap();
        save(&path, &request).unwrap();
        assert!(matches!(
            save(&path, &request),
            Err(AppError::Conflict { .. })
        ));
        let changed = read(&path).unwrap();
        assert_eq!(
            changed.initial_values.unwrap().timeout_ms.as_deref(),
            Some("2000")
        );
        restore(&path, &changed.revision).unwrap();
        let restored: Value = crate::config::read_json_file(&path).unwrap();
        assert_eq!(restored["env"][TIMEOUT], "2000");
        assert_eq!(restored["env"][TRAFFIC], "1");
        assert_eq!(restored["env"]["ANTHROPIC_AUTH_TOKEN"], "new");
        assert_eq!(restored["external"], 42);
        assert_eq!(restored["permissions"], original["permissions"]);
        cleanup(home);
    }

    #[test]
    fn request_rejects_omitted_or_unrelated_fields() {
        assert!(serde_json::from_value::<SaveRequest>(
            json!({"revision": "x", "values": {"timeoutMs": "1"}})
        )
        .is_err());
        assert!(serde_json::from_value::<SaveRequest>(json!({"revision": "x", "values": {"timeoutMs": "1", "nonessentialTraffic": null, "ANTHROPIC_AUTH_TOKEN": "key"}})).is_err());
        assert!(serde_json::from_value::<SaveRequest>(
            json!({"revision": "x", "values": {"timeoutMs": null, "nonessentialTraffic": null}})
        )
        .is_ok());
    }

    #[test]
    fn missing_file_defaults_noops_invalid_values_and_malformed_env_are_safe() {
        let _env = acquire_env_lock();
        let home = setup_test_home();
        let path = crate::config::get_claude_settings_path();
        let initial = read(&path).unwrap();
        save(
            &path,
            &SaveRequest {
                revision: initial.revision.clone(),
                values: UserValues::default(),
            },
        )
        .unwrap();
        assert!(!path.exists());
        assert!(save(
            &path,
            &SaveRequest {
                revision: initial.revision.clone(),
                values: UserValues {
                    timeout_ms: Some("0".into()),
                    nonessential_traffic: None
                }
            }
        )
        .is_err());
        assert!(!path.exists());
        save(
            &path,
            &SaveRequest {
                revision: initial.revision,
                values: UserValues {
                    timeout_ms: Some("10".into()),
                    nonessential_traffic: None,
                },
            },
        )
        .unwrap();
        let loaded = read(&path).unwrap();
        assert_eq!(loaded.initial_values, Some(UserValues::default()));
        restore(&path, &loaded.revision).unwrap();
        assert_eq!(read(&path).unwrap().values, UserValues::default());
        std::fs::write(&path, r#"{"env": ["external"]}"#).unwrap();
        assert!(read(&path).is_err());
        let bytes = std::fs::read(&path).unwrap();
        assert!(save(
            &path,
            &SaveRequest {
                revision: "stale".into(),
                values: UserValues::default()
            }
        )
        .is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        cleanup(home);
    }
}
