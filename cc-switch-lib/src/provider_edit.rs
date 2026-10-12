//! Provider editor's shared, pure three-way merge plan.
use crate::{database::Provider, error::AppError, live};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Policy {
    #[default]
    Refuse,
    KeepMine,
    KeepTheirs,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditRequest {
    pub provider: Provider,
    pub base: Provider,
    #[serde(default)]
    pub policy: Policy,
    pub resolution_revision: Option<String>,
}
pub struct Plan {
    pub provider: Provider,
    pub live_after: Value,
    pub revision: String,
    pub record_keys: Vec<String>,
    pub live_keys: Vec<String>,
}

pub fn revision(provider: &Provider, current: &Value, applied: bool) -> String {
    hex::encode(Sha256::digest(
        serde_json::to_vec(&(provider, current, applied)).expect("JSON serializes"),
    ))
}
fn merge(
    base: Option<&Value>,
    desired: Option<&Value>,
    current: Option<&Value>,
    path: &str,
    policy: Policy,
    conflicts: &mut Vec<String>,
) -> Option<Value> {
    if desired == base {
        return current.cloned();
    }
    if let (Some(Value::Object(b)), Some(Value::Object(d)), Some(Value::Object(c))) =
        (base, desired, current)
    {
        let keys: std::collections::BTreeSet<_> = b.keys().chain(d.keys()).collect();
        let mut result = c.clone();
        for key in keys {
            let next = format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"));
            match merge(b.get(key), d.get(key), c.get(key), &next, policy, conflicts) {
                Some(value) => {
                    result.insert(key.clone(), value);
                }
                None => {
                    result.remove(key);
                }
            }
        }
        return Some(Value::Object(result));
    }
    if current != base && current != desired {
        conflicts.push(path.into());
        if !matches!(policy, Policy::KeepMine) {
            return current.cloned();
        }
    }
    desired.cloned()
}
fn changed(base: Option<&Value>, after: Option<&Value>, path: &str, keys: &mut Vec<String>) {
    if base == after {
        return;
    }
    if let (Some(Value::Object(b)), Some(Value::Object(a))) = (base, after) {
        let names: std::collections::BTreeSet<_> = b.keys().chain(a.keys()).collect();
        for key in names {
            changed(
                b.get(key),
                a.get(key),
                &format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")),
                keys,
            );
        }
    } else {
        keys.push(path.into());
    }
}
/// No disk reads or writes. The caller controls current-provider/routing context.
pub fn prepare(
    request: &EditRequest,
    old: &Provider,
    current: &Value,
    applied: bool,
    port: u16,
) -> Result<Plan, AppError> {
    if request.provider.id != request.base.id || request.provider.id != old.id {
        return Err(AppError::InvalidInput(
            "Provider identity cannot change during editing".into(),
        ));
    }
    let observed = revision(old, current, applied);
    if !matches!(request.policy, Policy::Refuse)
        && request.resolution_revision.as_deref() != Some(observed.as_str())
    {
        return Err(AppError::ProviderEditConflict {
            keys: vec![],
            revision: observed,
        });
    }
    let base =
        serde_json::to_value(&request.base).map_err(|source| AppError::JsonSerialize { source })?;
    let desired = serde_json::to_value(&request.provider)
        .map_err(|source| AppError::JsonSerialize { source })?;
    let existing =
        serde_json::to_value(old).map_err(|source| AppError::JsonSerialize { source })?;
    let mut conflicts = vec![];
    let result = merge(
        Some(&base),
        Some(&desired),
        Some(&existing),
        "record",
        request.policy,
        &mut conflicts,
    )
    .expect("provider object");
    let mut provider: Provider = serde_json::from_value(result.clone())
        .map_err(|source| AppError::JsonSerialize { source })?;
    crate::providers::normalize_provider_schema(&mut provider);
    validate(&provider)?;
    let mut live_after = current.clone();
    if applied {
        if !current.is_object() || current.get("env").is_some_and(|env| !env.is_object()) {
            return Err(AppError::Config(
                "Live settings and env must be objects".into(),
            ));
        }
        let before = live::settings_for_live(old, port, false);
        let after = live::settings_for_live(&provider, port, false);
        let empty = serde_json::Map::new();
        let b = before
            .get("env")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let a = after
            .get("env")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let c = current
            .get("env")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let keys: std::collections::BTreeSet<_> = b
            .keys()
            .chain(a.keys())
            .filter(|key| live::floor::claude_owned_env(key))
            .collect();
        let mut env = c.clone();
        for key in keys {
            let path = format!("live/env/{}", key.replace('~', "~0").replace('/', "~1"));
            if matches!(request.policy, Policy::KeepTheirs)
                && b.get(key) != a.get(key)
                && c.get(key) != b.get(key)
                && c.get(key) != a.get(key)
            {
                // Keeping a live connection field also retains it in the saved provider,
                // so a later switch does not silently reapply the rejected draft value.
                if provider.settings_config.get("env").is_none() {
                    provider.settings_config["env"] = serde_json::json!({});
                }
                let stored = provider.settings_config["env"]
                    .as_object_mut()
                    .expect("validated provider env");
                match c.get(key) {
                    Some(value) => {
                        stored.insert(key.clone(), value.clone());
                    }
                    None => {
                        stored.remove(key);
                    }
                }
            }
            match merge(
                b.get(key),
                a.get(key),
                c.get(key),
                &path,
                request.policy,
                &mut conflicts,
            ) {
                Some(value) => {
                    env.insert(key.clone(), value);
                }
                None => {
                    env.remove(key);
                }
            }
        }
        live_after["env"] = Value::Object(env);
    }
    if !conflicts.is_empty() && matches!(request.policy, Policy::Refuse) {
        return Err(AppError::ProviderEditConflict {
            keys: conflicts,
            revision: observed,
        });
    }
    let mut record_keys = vec![];
    let mut live_keys = vec![];
    changed(
        Some(&existing),
        Some(&serde_json::to_value(&provider).expect("provider serializes")),
        "record",
        &mut record_keys,
    );
    changed(Some(current), Some(&live_after), "live", &mut live_keys);
    Ok(Plan {
        provider,
        live_after,
        revision: observed,
        record_keys,
        live_keys,
    })
}
pub fn validate(provider: &Provider) -> Result<(), AppError> {
    if provider.id.trim().is_empty()
        || provider.name.trim().is_empty()
        || !provider.settings_config.is_object()
        || provider
            .settings_config
            .get("env")
            .is_some_and(|env| !env.is_object())
    {
        return Err(AppError::InvalidInput(
            "Provider ID, name and an object configuration are required".into(),
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn provider() -> Provider {
        serde_json::from_value(json!({"id":"one","name":"One","settingsConfig":{"env":{"ANTHROPIC_AUTH_TOKEN":"old","ANTHROPIC_MODEL":"a"}},"meta":{},"inFailoverQueue":false})).unwrap()
    }
    #[test]
    fn merges_only_edited_fields_and_preserves_external_live_and_record_changes() {
        let base = provider();
        let mut old = base.clone();
        old.notes = Some("external".into());
        let mut draft = base.clone();
        draft.name = "Renamed".into();
        draft.settings_config["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("new");
        let request = EditRequest {
            provider: draft,
            base,
            policy: Policy::Refuse,
            resolution_revision: None,
        };
        let live = json!({"env":{"ANTHROPIC_AUTH_TOKEN":"old","ANTHROPIC_MODEL":"outside","API_TIMEOUT_MS":"777"},"permissions":{"allow":["Read"]}});
        let plan = prepare(&request, &old, &live, true, 0).unwrap();
        assert_eq!(plan.provider.notes.as_deref(), Some("external"));
        assert_eq!(plan.live_after["env"]["ANTHROPIC_MODEL"], "outside");
        assert_eq!(plan.live_after["env"]["API_TIMEOUT_MS"], "777");
        assert_eq!(plan.live_after["permissions"], live["permissions"]);
    }
    #[test]
    fn conflict_resolution_rechecks_observed_state_and_ignores_live_when_routed() {
        let base = provider();
        let mut draft = base.clone();
        draft.settings_config["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("mine");
        let mut request = EditRequest {
            provider: draft,
            base: base.clone(),
            policy: Policy::Refuse,
            resolution_revision: None,
        };
        let live = json!({"env":{"ANTHROPIC_AUTH_TOKEN":"external"}});
        assert!(
            matches!(prepare(&request, &base, &live, true, 0), Err(AppError::ProviderEditConflict { keys, .. }) if keys == ["live/env/ANTHROPIC_AUTH_TOKEN"])
        );
        request.policy = Policy::KeepTheirs;
        request.resolution_revision = Some(revision(&base, &live, true));
        let kept = prepare(&request, &base, &live, true, 0).unwrap();
        assert_eq!(kept.live_after, live);
        assert_eq!(
            kept.provider.settings_config["env"]["ANTHROPIC_AUTH_TOKEN"],
            "external"
        );
        request.policy = Policy::KeepMine;
        assert_eq!(
            prepare(&request, &base, &live, true, 0).unwrap().live_after["env"]
                ["ANTHROPIC_AUTH_TOKEN"],
            "mine"
        );
        assert!(prepare(&request, &base, &json!({}), true, 0).is_err());
        request.policy = Policy::Refuse;
        let plan = prepare(&request, &base, &live, false, 0).unwrap();
        assert!(plan.live_keys.is_empty());
        assert_eq!(plan.live_after, live);
    }
}
