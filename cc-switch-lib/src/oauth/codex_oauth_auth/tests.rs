use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

use super::*;
use super::types::POLLING_SAFETY_MARGIN_SECS;

#[test]
fn test_parse_interval_number() {
    let v = serde_json::Value::Number(serde_json::Number::from(5));
    assert_eq!(parse_interval(Some(&v)), 5 + POLLING_SAFETY_MARGIN_SECS);
}

#[test]
fn test_parse_interval_string() {
    let v = serde_json::Value::String("10".to_string());
    assert_eq!(parse_interval(Some(&v)), 10 + POLLING_SAFETY_MARGIN_SECS);
}

#[test]
fn test_parse_interval_default() {
    assert_eq!(parse_interval(None), 5 + POLLING_SAFETY_MARGIN_SECS);
}

#[test]
fn test_parse_interval_min() {
    let v = serde_json::Value::Number(serde_json::Number::from(0));
    // 0 应被提升到 1
    assert_eq!(parse_interval(Some(&v)), 1 + POLLING_SAFETY_MARGIN_SECS);
}

#[test]
fn test_compute_expires_at_ms() {
    let result = compute_expires_at_ms(Some(3600));
    let now = chrono::Utc::now().timestamp_millis();
    // 应在未来约 3600 秒处（允许少量误差）
    assert!(result > now + 3500 * 1000);
    assert!(result < now + 3700 * 1000);
}

#[test]
fn test_compute_expires_at_ms_default() {
    let result = compute_expires_at_ms(None);
    let now = chrono::Utc::now().timestamp_millis();
    assert!(result > now);
}

#[test]
fn test_cached_token_expiring_soon() {
    let now = chrono::Utc::now().timestamp_millis();
    // 30 秒后过期 - 在缓冲期内
    let expiring = CachedAccessToken {
        token: "t".to_string(),
        expires_at_ms: now + 30_000,
    };
    assert!(expiring.is_expiring_soon());

    // 1 小时后过期 - 不在缓冲期内
    let valid = CachedAccessToken {
        token: "t".to_string(),
        expires_at_ms: now + 3_600_000,
    };
    assert!(!valid.is_expiring_soon());
}

#[test]
fn test_parse_jwt_claims_invalid() {
    assert!(parse_jwt_claims("not-a-jwt").is_none());
    assert!(parse_jwt_claims("only.two").is_none());
}

#[test]
fn test_parse_jwt_claims_valid() {
    // Header: {"alg":"none"}
    // Payload: {"chatgpt_account_id":"acc-123","email":"test@example.com"}
    // Signature: empty
    let header = URL_SAFE_NO_PAD.encode(b"{\"alg\":\"none\"}");
    let payload = URL_SAFE_NO_PAD
        .encode(b"{\"chatgpt_account_id\":\"acc-123\",\"email\":\"test@example.com\"}");
    let jwt = format!("{header}.{payload}.");
    let claims = parse_jwt_claims(&jwt).unwrap();
    assert_eq!(claims.chatgpt_account_id.as_deref(), Some("acc-123"));
    assert_eq!(claims.email.as_deref(), Some("test@example.com"));
}

#[test]
fn test_parse_jwt_claims_organizations_fallback() {
    let header = URL_SAFE_NO_PAD.encode(b"{\"alg\":\"none\"}");
    let payload = URL_SAFE_NO_PAD.encode(b"{\"organizations\":[{\"id\":\"org-456\"}]}");
    let jwt = format!("{header}.{payload}.");
    let claims = parse_jwt_claims(&jwt).unwrap();
    assert_eq!(
        claims
            .organizations
            .first()
            .and_then(|o| o.id.clone())
            .as_deref(),
        Some("org-456")
    );
}

#[tokio::test]
async fn test_manager_initial_state() {
    let temp = tempfile::tempdir().unwrap();
    let manager = CodexOAuthManager::new(temp.path().to_path_buf());
    assert!(!manager.is_authenticated().await);
    assert!(manager.list_accounts().await.is_empty());
}

#[tokio::test]
async fn test_manager_save_and_load() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().to_path_buf();

    // Manually inject an account through internal methods
    {
        let manager = CodexOAuthManager::new(path.clone());
        manager
            .add_account_internal(
                "acc-123".to_string(),
                "rt-secret".to_string(),
                Some("user@example.com".to_string()),
            )
            .await
            .unwrap();
    }

    // New manager should load from disk
    let manager2 = CodexOAuthManager::new(path);
    let accounts = manager2.list_accounts().await;
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id, "acc-123");
}

#[tokio::test]
async fn test_remove_account() {
    let temp = tempfile::tempdir().unwrap();
    let manager = CodexOAuthManager::new(temp.path().to_path_buf());

    manager
        .add_account_internal(
            "acc-123".to_string(),
            "rt".to_string(),
            Some("a@example.com".to_string()),
        )
        .await
        .unwrap();
    manager
        .add_account_internal(
            "acc-456".to_string(),
            "rt2".to_string(),
            Some("b@example.com".to_string()),
        )
        .await
        .unwrap();

    manager.remove_account("acc-123").await.unwrap();
    let accounts = manager.list_accounts().await;
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id, "acc-456");
}
