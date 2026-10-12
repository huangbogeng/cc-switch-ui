//! Managed Codex model discovery, adapted from cc-switch at d0b57827.
//! Copyright (c) 2025 Jason Young. MIT; see docs/licenses/cc-switch-MIT.txt.

use super::CodexOAuthManager;
use crate::providers::FetchedModel;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

const MODELS_URL: &str = "https://chatgpt.com/backend-api/codex/models";
// Match upstream's catalog client identity; catalog visibility is version gated.
const CLIENT_VERSION: &str = "0.159.0";
const ORIGINATOR: &str = "codex_cli_rs";

impl CodexOAuthManager {
    /// Read the selected managed account's catalog without requiring a running route.
    pub async fn fetch_models(
        &self,
        account_id: Option<&str>,
    ) -> Result<Vec<FetchedModel>, String> {
        let (token, account_id) = self.models_credentials(account_id).await?;
        let client = self.http_client.read().await.clone();
        fetch_catalog(&client, MODELS_URL, &token, &account_id).await
    }

    async fn models_credentials(
        &self,
        account_id: Option<&str>,
    ) -> Result<(String, String), String> {
        let resolved = match account_id.map(str::trim).filter(|id| !id.is_empty()) {
            Some(id) => id.to_string(),
            None => self.default_account_id().await.ok_or_else(|| {
                "Connect a Codex OAuth account before fetching models.".to_string()
            })?,
        };
        // An explicit stale binding must not silently use another account.
        if !self.accounts.read().await.contains_key(&resolved) {
            return Err(
                "Selected Codex OAuth account is unavailable. Choose a connected account."
                    .to_string(),
            );
        }
        let token = self
            .get_valid_token_for_account(&resolved)
            .await
            .map_err(|_| {
                "Codex OAuth token unavailable. Sign in again on the OAuth page.".to_string()
            })?;
        // This manager stores the upstream ChatGPT account ID as the account key,
        // matching the generation route's chatgpt-account-id header.
        Ok((token, resolved))
    }
}

fn build_request(
    client: &Client,
    url: &str,
    token: &str,
    account_id: &str,
) -> reqwest::RequestBuilder {
    client
        .get(url)
        .query(&[("client_version", CLIENT_VERSION)])
        .bearer_auth(token)
        .header("originator", ORIGINATOR)
        .header("version", CLIENT_VERSION)
        .header("chatgpt-account-id", account_id)
        .timeout(Duration::from_secs(15))
}

async fn fetch_catalog(
    client: &Client,
    url: &str,
    token: &str,
    account_id: &str,
) -> Result<Vec<FetchedModel>, String> {
    let response = build_request(client, url, token, account_id)
        .send()
        .await
        .map_err(|_| {
            "Codex model catalog request failed. Check the network or proxy settings.".to_string()
        })?;
    let status = response.status();
    if !status.is_success() {
        // Upstream bodies can contain account details; expose only status and action.
        return Err(
            if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
                format!(
                    "Codex model catalog returned HTTP {}. Check account access or sign in again.",
                    status.as_u16()
                )
            } else {
                format!(
                    "Codex model catalog returned HTTP {}. Try again later.",
                    status.as_u16()
                )
            },
        );
    }
    let value: Value = response
        .json()
        .await
        .map_err(|_| "Codex model catalog returned invalid JSON.".to_string())?;
    Ok(parse_models(value))
}

fn parse_models(value: Value) -> Vec<FetchedModel> {
    let entries = value
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| value.get("models").and_then(Value::as_array))
        .or_else(|| value.get("items").and_then(Value::as_array))
        .or_else(|| value.as_array());

    let mut models = Vec::new();

    if let Some(entries) = entries {
        for entry in entries {
            push_model_entry(&mut models, entry, None);
        }
    }

    if let Some(model_map) = value.get("models").and_then(Value::as_object) {
        for (key, entry) in model_map {
            push_model_entry(&mut models, entry, Some(key));
        }
    }

    models.sort_by(|a, b| a.id.cmp(&b.id));
    models.dedup_by(|a, b| a.id == b.id);
    models
}

fn push_model_entry(models: &mut Vec<FetchedModel>, entry: &Value, fallback_id: Option<&str>) {
    if let Some(id) = entry.as_str().map(str::trim).filter(|id| !id.is_empty()) {
        models.push(FetchedModel {
            id: id.to_string(),
            owned_by: Some("Codex".to_string()),
        });
        return;
    }

    let Some(obj) = entry.as_object() else {
        if let Some(id) = fallback_id.map(str::trim).filter(|id| !id.is_empty()) {
            models.push(FetchedModel {
                id: id.to_string(),
                owned_by: Some("Codex".to_string()),
            });
        }
        return;
    };

    let Some(id) = string_field(obj, &["slug", "id", "model", "name"]).or_else(|| {
        fallback_id
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_string)
    }) else {
        return;
    };
    let owned_by = string_field(
        obj,
        &[
            "owned_by", "ownedBy", "provider", "vendor", "category", "owner",
        ],
    )
    .or_else(|| Some("Codex".to_string()));

    models.push(FetchedModel { id, owned_by });
}

fn string_field(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| obj.get(*key))
        .filter_map(Value::as_str)
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::super::types::{compute_expires_at_ms, CachedAccessToken};
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn selected_account_and_default_use_their_own_cached_tokens() {
        let home = tempfile::tempdir().unwrap();
        let manager = CodexOAuthManager::new(home.path().to_path_buf());
        for id in ["fixture-a", "fixture-b"] {
            manager
                .add_account_internal(id.to_string(), "unused-refresh".to_string(), None)
                .await
                .unwrap();
            manager.access_tokens.write().await.insert(
                id.to_string(),
                CachedAccessToken {
                    token: format!("token-{id}"),
                    expires_at_ms: compute_expires_at_ms(Some(3600)),
                },
            );
        }
        manager.set_default_account("fixture-a").await.unwrap();
        assert_eq!(
            manager.models_credentials(None).await.unwrap(),
            ("token-fixture-a".into(), "fixture-a".into())
        );
        assert_eq!(
            manager
                .models_credentials(Some(" fixture-b "))
                .await
                .unwrap(),
            ("token-fixture-b".into(), "fixture-b".into())
        );
        assert!(manager
            .models_credentials(Some("missing"))
            .await
            .unwrap_err()
            .contains("Selected"));
        manager.set_default_account("fixture-b").await.unwrap();
        assert_eq!(
            manager.models_credentials(Some(" ")).await.unwrap().1,
            "fixture-b"
        );
    }

    #[tokio::test]
    async fn no_account_fails_before_any_network_request() {
        let home = tempfile::tempdir().unwrap();
        let manager = CodexOAuthManager::new(home.path().to_path_buf());
        assert!(manager
            .fetch_models(None)
            .await
            .unwrap_err()
            .contains("Connect"));
    }

    #[test]
    fn catalog_response_shapes_are_sorted_and_deduplicated() {
        for value in [
            json!({"data":[{"id":"beta","owned_by":"openai"},{"model":"alpha"},{"id":"beta"}]}),
            json!({"models":[{"slug":"beta"},"alpha","beta"]}),
            json!({"items":[{"name":"beta"},{"id":"alpha"}]}),
            json!(["beta", "alpha"]),
            json!({"models":{"beta":{},"alpha":{"slug":"alpha"}}}),
        ] {
            assert_eq!(
                parse_models(value)
                    .iter()
                    .map(|m| m.id.as_str())
                    .collect::<Vec<_>>(),
                ["alpha", "beta"]
            );
        }
        assert!(parse_models(json!({"models":[null,{}," "]})).is_empty());
    }

    #[tokio::test]
    async fn real_http_catalog_uses_oauth_headers_and_redacts_error_bodies() {
        use axum::{
            extract::Query,
            http::{HeaderMap, StatusCode},
            routing::get,
            Json, Router,
        };
        use std::collections::HashMap;
        let app = Router::new()
            .route(
                "/catalog",
                get(
                    |headers: HeaderMap, Query(query): Query<HashMap<String, String>>| async move {
                        assert_eq!(headers["authorization"], "Bearer fixture-token");
                        assert_eq!(headers["chatgpt-account-id"], "fixture-workspace");
                        assert_eq!(headers["originator"], "codex_cli_rs");
                        assert_eq!(headers["version"], "0.159.0");
                        assert_eq!(query["client_version"], "0.159.0");
                        Json(json!({"models":[{"slug":"catalog-model"}]}))
                    },
                ),
            )
            .route(
                "/rejected",
                get(|| async {
                    (
                        StatusCode::UNAUTHORIZED,
                        "PRIVATE fixture-token fixture-workspace",
                    )
                }),
            )
            .route("/invalid", get(|| async { "invalid-json PRIVATE" }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = Client::builder().no_proxy().build().unwrap();
        let models = fetch_catalog(
            &client,
            &format!("http://{addr}/catalog"),
            "fixture-token",
            "fixture-workspace",
        )
        .await
        .unwrap();
        assert_eq!(models[0].id, "catalog-model");
        for path in ["rejected", "invalid"] {
            let error = fetch_catalog(
                &client,
                &format!("http://{addr}/{path}"),
                "fixture-token",
                "fixture-workspace",
            )
            .await
            .unwrap_err();
            assert!(!error.contains("PRIVATE"));
            assert!(!error.contains("fixture-token"));
            assert!(!error.contains("fixture-workspace"));
        }
        // The only public production entry point uses the fixed upstream URL.
        let request = build_request(&client, MODELS_URL, "fixture-token", "fixture-workspace")
            .build()
            .unwrap();
        assert_eq!(request.url().host_str(), Some("chatgpt.com"));
        assert_eq!(request.url().path(), "/backend-api/codex/models");
        task.abort();
    }
}
