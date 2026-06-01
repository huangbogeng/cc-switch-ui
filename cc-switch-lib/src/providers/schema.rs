use crate::database::Provider;
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeyField {
    AnthropicAuthToken,
    AnthropicApiKey,
}

impl ApiKeyField {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AnthropicAuthToken => "ANTHROPIC_AUTH_TOKEN",
            Self::AnthropicApiKey => "ANTHROPIC_API_KEY",
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::AnthropicAuthToken => Self::AnthropicApiKey,
            Self::AnthropicApiKey => Self::AnthropicAuthToken,
        }
    }
}

fn parse_api_key_field(v: Option<&str>) -> Option<ApiKeyField> {
    match v {
        Some("ANTHROPIC_API_KEY") => Some(ApiKeyField::AnthropicApiKey),
        Some("ANTHROPIC_AUTH_TOKEN") => Some(ApiKeyField::AnthropicAuthToken),
        _ => None,
    }
}

fn env_obj(settings_config: &Value) -> Option<&Map<String, Value>> {
    settings_config.get("env").and_then(Value::as_object)
}

fn env_obj_mut(settings_config: &mut Value) -> Option<&mut Map<String, Value>> {
    settings_config.get_mut("env").and_then(Value::as_object_mut)
}

fn non_empty_env_str(env: &Map<String, Value>, key: &str) -> Option<String> {
    env.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

fn provider_base_url(provider: &Provider) -> Option<&str> {
    provider
        .settings_config
        .get("baseUrl")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .or_else(|| {
            env_obj(&provider.settings_config)
                .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
        })
}

fn is_local_base_url(base_url: &str) -> bool {
    let normalized = base_url.trim().to_ascii_lowercase();
    normalized.starts_with("http://127.0.0.1")
        || normalized.starts_with("https://127.0.0.1")
        || normalized.starts_with("http://localhost")
        || normalized.starts_with("https://localhost")
        || normalized.starts_with("http://0.0.0.0")
        || normalized.starts_with("https://0.0.0.0")
}

pub fn resolve_provider_api_key_field(provider: &Provider) -> ApiKeyField {
    let declared = provider
        .meta
        .get("apiKeyField")
        .and_then(Value::as_str)
        .and_then(|v| parse_api_key_field(Some(v)));
    if let Some(v) = declared {
        return v;
    }

    let env = env_obj(&provider.settings_config);
    if env.and_then(|v| v.get("ANTHROPIC_API_KEY")).is_some() {
        return ApiKeyField::AnthropicApiKey;
    }
    ApiKeyField::AnthropicAuthToken
}

pub fn resolve_provider_api_key(provider: &Provider) -> Option<String> {
    let preferred = resolve_provider_api_key_field(provider);
    if let Some(env) = env_obj(&provider.settings_config) {
        if let Some(v) = non_empty_env_str(env, preferred.as_str()) {
            return Some(v);
        }
        if let Some(v) = non_empty_env_str(env, preferred.other().as_str()) {
            return Some(v);
        }
    }

    for key in ["apiKey", "api_key", "authToken", "token"] {
        if let Some(v) = provider
            .settings_config
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            return Some(v.to_string());
        }
    }
    None
}

pub fn provider_allows_empty_api_key(provider: &Provider) -> bool {
    provider_base_url(provider).is_some_and(is_local_base_url)
}

pub fn resolve_managed_account_id(provider: &Provider) -> Option<String> {
    provider
        .meta
        .get("authBinding")
        .and_then(|value| {
            value
                .get("accountId")
                .and_then(Value::as_str)
                .or_else(|| value.as_str())
        })
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

pub fn normalize_provider_schema(provider: &mut Provider) {
    let keep = resolve_provider_api_key_field(provider);
    let drop = keep.other();

    let Some(env) = env_obj_mut(&mut provider.settings_config) else {
        return;
    };

    let keep_empty = env
        .get(keep.as_str())
        .and_then(Value::as_str)
        .map_or(true, |s| s.trim().is_empty());
    if keep_empty {
        if let Some(other_val) = env
            .get(drop.as_str())
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            env.insert(keep.as_str().to_string(), json!(other_val));
        }
    }

    let keep_empty_after_merge = env
        .get(keep.as_str())
        .and_then(Value::as_str)
        .map_or(true, |s| s.trim().is_empty());
    if keep_empty_after_merge {
        env.remove(keep.as_str());
    }

    let drop_empty = env
        .get(drop.as_str())
        .and_then(Value::as_str)
        .map_or(true, |s| s.trim().is_empty());
    if drop_empty {
        env.remove(drop.as_str());
    }

    if !provider.meta.is_object() {
        provider.meta = json!({});
    }
    if let Some(meta) = provider.meta.as_object_mut() {
        meta.insert("apiKeyField".to_string(), json!(keep.as_str()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider_with(env: Value, meta: Value) -> Provider {
        Provider {
            id: "p".to_string(),
            name: "P".to_string(),
            settings_config: json!({ "env": env }),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            icon: None,
            icon_color: None,
            meta,
            in_failover_queue: false,
        }
    }

    #[test]
    fn resolve_api_key_field_prefers_env_api_key_for_legacy_provider() {
        let p = provider_with(json!({"ANTHROPIC_API_KEY":"k"}), json!({}));
        assert_eq!(
            resolve_provider_api_key_field(&p),
            ApiKeyField::AnthropicApiKey
        );
    }

    #[test]
    fn normalize_schema_keeps_non_empty_alternate_key() {
        let mut p = provider_with(
            json!({
                "ANTHROPIC_AUTH_TOKEN": "new",
                "ANTHROPIC_API_KEY": "old"
            }),
            json!({"apiKeyField":"ANTHROPIC_AUTH_TOKEN"}),
        );
        normalize_provider_schema(&mut p);
        let env = p.settings_config.get("env").and_then(Value::as_object).unwrap();
        assert_eq!(
            env.get("ANTHROPIC_AUTH_TOKEN").and_then(Value::as_str),
            Some("new")
        );
        assert_eq!(
            env.get("ANTHROPIC_API_KEY").and_then(Value::as_str),
            Some("old")
        );
    }

    #[test]
    fn normalize_schema_strips_empty_api_key_fields() {
        let mut p = provider_with(
            json!({
                "ANTHROPIC_AUTH_TOKEN": "",
                "ANTHROPIC_API_KEY": "   "
            }),
            json!({"apiKeyField":"ANTHROPIC_AUTH_TOKEN"}),
        );
        normalize_provider_schema(&mut p);
        let env = p.settings_config.get("env").and_then(Value::as_object).unwrap();
        assert_eq!(env.get("ANTHROPIC_AUTH_TOKEN"), None);
        assert_eq!(env.get("ANTHROPIC_API_KEY"), None);
    }

    #[test]
    fn resolve_managed_account_id_supports_object_shape() {
        let p = provider_with(
            json!({}),
            json!({"authBinding":{"source":"managed_account","accountId":"acc-1"}}),
        );
        assert_eq!(resolve_managed_account_id(&p).as_deref(), Some("acc-1"));
    }

    #[test]
    fn local_base_url_allows_empty_api_key() {
        let p = provider_with(
            json!({"ANTHROPIC_BASE_URL":"http://127.0.0.1:11434/v1"}),
            json!({}),
        );
        assert!(provider_allows_empty_api_key(&p));
    }

    #[test]
    fn remote_base_url_does_not_allow_empty_api_key() {
        let p = provider_with(
            json!({"ANTHROPIC_BASE_URL":"https://api.openrouter.ai/v1"}),
            json!({}),
        );
        assert!(!provider_allows_empty_api_key(&p));
    }
}
