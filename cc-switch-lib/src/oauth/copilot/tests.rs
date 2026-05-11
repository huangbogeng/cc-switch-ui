use super::types::{
    normalize_github_domain, composite_account_id, copilot_api_base, CopilotAuthStore,
    CopilotAuthError, GitHubAccountData,
};
use super::{CopilotAuthManager, CopilotAuthStatus, CopilotModel, CopilotToken,
    GitHubAccount, GitHubUser, DEFAULT_GITHUB_DOMAIN};
use std::collections::HashMap;
use tempfile::tempdir;

#[test]
fn test_copilot_token_expiry() {
    let now = chrono::Utc::now().timestamp();

    // 未过期的 token (1小时后过期，不在60秒缓冲期内)
    let token = CopilotToken {
        token: "test".to_string(),
        expires_at: now + 3600,
    };
    assert!(!token.is_expiring_soon());

    // 即将过期的 token (30秒后过期，在60秒缓冲期内)
    let token = CopilotToken {
        token: "test".to_string(),
        expires_at: now + 30,
    };
    assert!(token.is_expiring_soon());

    // 已过期的 token (也在缓冲期内)
    let token = CopilotToken {
        token: "test".to_string(),
        expires_at: now - 100,
    };
    assert!(token.is_expiring_soon());
}

#[test]
fn test_auth_status_serialization() {
    let status = CopilotAuthStatus {
        accounts: vec![GitHubAccount {
            id: "12345".to_string(),
            login: "testuser".to_string(),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            authenticated_at: 1234567890,
            github_domain: DEFAULT_GITHUB_DOMAIN.to_string(),
        }],
        default_account_id: Some("12345".to_string()),
        migration_error: None,
        authenticated: true,
        username: Some("testuser".to_string()),
        expires_at: Some(1234567890),
    };

    let json = serde_json::to_string(&status).unwrap();
    let parsed: CopilotAuthStatus = serde_json::from_str(&json).unwrap();

    assert!(parsed.authenticated);
    assert_eq!(parsed.default_account_id, Some("12345".to_string()));
    assert_eq!(parsed.username, Some("testuser".to_string()));
    assert_eq!(parsed.expires_at, Some(1234567890));
    assert_eq!(parsed.accounts.len(), 1);
    assert_eq!(parsed.accounts[0].id, "12345");
    assert_eq!(parsed.accounts[0].login, "testuser");
}

#[test]
fn test_multi_account_store_serialization() {
    let mut accounts = HashMap::new();
    accounts.insert(
        "12345".to_string(),
        GitHubAccountData {
            github_token: "gho_test_token".to_string(),
            user: GitHubUser {
                login: "alice".to_string(),
                id: 12345,
                avatar_url: Some("https://example.com/alice.png".to_string()),
            },
            authenticated_at: 1700000000,
            github_domain: DEFAULT_GITHUB_DOMAIN.to_string(),
        },
    );
    accounts.insert(
        "67890".to_string(),
        GitHubAccountData {
            github_token: "gho_test_token_2".to_string(),
            user: GitHubUser {
                login: "bob".to_string(),
                id: 67890,
                avatar_url: None,
            },
            authenticated_at: 1700000001,
            github_domain: DEFAULT_GITHUB_DOMAIN.to_string(),
        },
    );

    let store = CopilotAuthStore {
        version: 3,
        accounts,
        default_account_id: Some("67890".to_string()),
        github_token: None,
        authenticated_at: None,
    };

    let json = serde_json::to_string_pretty(&store).unwrap();
    let parsed: CopilotAuthStore = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.version, 3);
    assert_eq!(parsed.default_account_id, Some("67890".to_string()));
    assert_eq!(parsed.accounts.len(), 2);
    assert!(parsed.accounts.contains_key("12345"));
    assert!(parsed.accounts.contains_key("67890"));
    assert_eq!(parsed.accounts["12345"].user.login, "alice");
    assert_eq!(parsed.accounts["67890"].user.login, "bob");
}

#[test]
fn test_legacy_format_detection() {
    // 旧格式（v1）
    let legacy_json = r#"{
        "github_token": "gho_legacy_token",
        "authenticated_at": 1700000000
    }"#;

    let store: CopilotAuthStore = serde_json::from_str(legacy_json).unwrap();
    assert_eq!(store.version, 0); // 默认值
    assert!(store.github_token.is_some());
    assert!(store.accounts.is_empty());
}

#[test]
fn test_github_account_from_data() {
    let data = GitHubAccountData {
        github_token: "gho_test".to_string(),
        user: GitHubUser {
            login: "testuser".to_string(),
            id: 99999,
            avatar_url: Some("https://example.com/avatar.png".to_string()),
        },
        authenticated_at: 1700000000,
        github_domain: DEFAULT_GITHUB_DOMAIN.to_string(),
    };

    let account = GitHubAccount::from(&data);
    assert_eq!(account.id, "99999");
    assert_eq!(account.login, "testuser");
    assert_eq!(
        account.avatar_url,
        Some("https://example.com/avatar.png".to_string())
    );
    assert_eq!(account.authenticated_at, 1700000000);
}

#[test]
fn test_fallback_default_account_prefers_latest_authenticated() {
    let mut accounts = HashMap::new();
    accounts.insert(
        "12345".to_string(),
        GitHubAccountData {
            github_token: "gho_test_token".to_string(),
            user: GitHubUser {
                login: "alice".to_string(),
                id: 12345,
                avatar_url: None,
            },
            authenticated_at: 1700000000,
            github_domain: DEFAULT_GITHUB_DOMAIN.to_string(),
        },
    );
    accounts.insert(
        "67890".to_string(),
        GitHubAccountData {
            github_token: "gho_test_token_2".to_string(),
            user: GitHubUser {
                login: "bob".to_string(),
                id: 67890,
                avatar_url: None,
            },
            authenticated_at: 1700000001,
            github_domain: DEFAULT_GITHUB_DOMAIN.to_string(),
        },
    );

    assert_eq!(
        CopilotAuthManager::fallback_default_account_id(&accounts),
        Some("67890".to_string())
    );
}

#[tokio::test]
async fn test_get_model_vendor_from_cache() {
    let temp_dir = tempdir().unwrap();
    let manager = CopilotAuthManager::new(temp_dir.path().to_path_buf());

    {
        let mut default_account_id = manager.default_account_id.write().await;
        *default_account_id = Some("12345".to_string());
    }
    {
        let mut accounts = manager.accounts.write().await;
        accounts.insert(
            "12345".to_string(),
            GitHubAccountData {
                github_token: "gho_test".to_string(),
                user: GitHubUser {
                    login: "alice".to_string(),
                    id: 12345,
                    avatar_url: None,
                },
                authenticated_at: 1700000000,
                github_domain: DEFAULT_GITHUB_DOMAIN.to_string(),
            },
        );
    }
    {
        let mut models = manager.copilot_models.write().await;
        models.insert(
            "12345".to_string(),
            vec![
                CopilotModel {
                    id: "gpt-5.4".to_string(),
                    name: "GPT-5.4".to_string(),
                    vendor: "OpenAI".to_string(),
                    model_picker_enabled: true,
                },
                CopilotModel {
                    id: "claude-sonnet-4".to_string(),
                    name: "Claude Sonnet 4".to_string(),
                    vendor: "Anthropic".to_string(),
                    model_picker_enabled: true,
                },
            ],
        );
    }

    let vendor = manager
        .get_model_vendor_for_account("12345", "gpt-5.4")
        .await
        .unwrap();
    assert_eq!(vendor.as_deref(), Some("OpenAI"));

    let default_vendor = manager.get_model_vendor("claude-sonnet-4").await.unwrap();
    assert_eq!(default_vendor.as_deref(), Some("Anthropic"));
}

#[tokio::test]
async fn test_get_api_endpoint_returns_cached_value() {
    let temp_dir = tempdir().unwrap();
    let manager = CopilotAuthManager::new(temp_dir.path().to_path_buf());

    // 手动设置 api_endpoints 缓存
    {
        let mut api_endpoints = manager.api_endpoints.write().await;
        api_endpoints.insert(
            "12345".to_string(),
            "https://copilot-api.enterprise.example.com".to_string(),
        );
    }

    let endpoint = manager.get_api_endpoint("12345").await;
    assert_eq!(endpoint, "https://copilot-api.enterprise.example.com");
}

#[tokio::test]
async fn test_get_api_endpoint_returns_default_when_not_cached() {
    let temp_dir = tempdir().unwrap();
    let manager = CopilotAuthManager::new(temp_dir.path().to_path_buf());

    let endpoint = manager.get_api_endpoint("99999").await;
    assert_eq!(endpoint, "https://api.githubcopilot.com");
}

#[tokio::test]
async fn test_get_default_api_endpoint_uses_default_account() {
    let temp_dir = tempdir().unwrap();
    let manager = CopilotAuthManager::new(temp_dir.path().to_path_buf());

    // 设置默认账号
    {
        let mut default_account_id = manager.default_account_id.write().await;
        *default_account_id = Some("12345".to_string());
    }
    // 添加账号数据
    {
        let mut accounts = manager.accounts.write().await;
        accounts.insert(
            "12345".to_string(),
            GitHubAccountData {
                github_token: "gho_test".to_string(),
                user: GitHubUser {
                    login: "alice".to_string(),
                    id: 12345,
                    avatar_url: None,
                },
                authenticated_at: 1700000000,
                github_domain: DEFAULT_GITHUB_DOMAIN.to_string(),
            },
        );
    }
    // 设置 API endpoint 缓存
    {
        let mut api_endpoints = manager.api_endpoints.write().await;
        api_endpoints.insert(
            "12345".to_string(),
            "https://copilot-api.corp.example.com".to_string(),
        );
    }

    let endpoint = manager.get_default_api_endpoint().await;
    assert_eq!(endpoint, "https://copilot-api.corp.example.com");
}

#[tokio::test]
async fn test_remove_account_clears_api_endpoint_cache() {
    let temp_dir = tempdir().unwrap();
    let manager = CopilotAuthManager::new(temp_dir.path().to_path_buf());

    // 添加账号数据
    {
        let mut accounts = manager.accounts.write().await;
        accounts.insert(
            "12345".to_string(),
            GitHubAccountData {
                github_token: "gho_test".to_string(),
                user: GitHubUser {
                    login: "alice".to_string(),
                    id: 12345,
                    avatar_url: None,
                },
                authenticated_at: 1700000000,
                github_domain: DEFAULT_GITHUB_DOMAIN.to_string(),
            },
        );
    }
    // 设置 API endpoint 缓存
    {
        let mut api_endpoints = manager.api_endpoints.write().await;
        api_endpoints.insert(
            "12345".to_string(),
            "https://copilot-api.enterprise.example.com".to_string(),
        );
    }

    // 确认缓存存在
    {
        let api_endpoints = manager.api_endpoints.read().await;
        assert!(api_endpoints.contains_key("12345"));
    }

    // 移除账号
    manager.remove_account("12345").await.unwrap();

    // 确认缓存已清理
    {
        let api_endpoints = manager.api_endpoints.read().await;
        assert!(!api_endpoints.contains_key("12345"));
    }
}

#[tokio::test]
async fn test_clear_auth_clears_all_api_endpoint_cache() {
    let temp_dir = tempdir().unwrap();
    let manager = CopilotAuthManager::new(temp_dir.path().to_path_buf());

    // 添加多个账号的 API endpoint 缓存
    {
        let mut api_endpoints = manager.api_endpoints.write().await;
        api_endpoints.insert(
            "12345".to_string(),
            "https://copilot-api.enterprise1.example.com".to_string(),
        );
        api_endpoints.insert(
            "67890".to_string(),
            "https://copilot-api.enterprise2.example.com".to_string(),
        );
    }

    // 确认缓存存在
    {
        let api_endpoints = manager.api_endpoints.read().await;
        assert_eq!(api_endpoints.len(), 2);
    }

    // 清除所有认证
    manager.clear_auth().await.unwrap();

    // 确认缓存已清空
    {
        let api_endpoints = manager.api_endpoints.read().await;
        assert!(api_endpoints.is_empty());
    }
}

#[tokio::test]
async fn test_clear_auth_cleans_memory_even_when_file_removal_fails() {
    let temp_dir = tempdir().unwrap();
    let manager = CopilotAuthManager::new(temp_dir.path().to_path_buf());

    // Create a directory at storage_path so remove_file fails
    std::fs::create_dir_all(&manager.storage_path).unwrap();

    {
        let mut accounts = manager.accounts.write().await;
        accounts.insert(
            "12345".to_string(),
            GitHubAccountData {
                github_token: "gho_test".to_string(),
                user: GitHubUser {
                    login: "alice".to_string(),
                    id: 12345,
                    avatar_url: None,
                },
                authenticated_at: 1700000000,
                github_domain: DEFAULT_GITHUB_DOMAIN.to_string(),
            },
        );
    }
    {
        let mut default_account_id = manager.default_account_id.write().await;
        *default_account_id = Some("12345".to_string());
    }
    {
        let mut api_endpoints = manager.api_endpoints.write().await;
        api_endpoints.insert(
            "12345".to_string(),
            "https://copilot-api.enterprise.example.com".to_string(),
        );
    }

    let result = manager.clear_auth().await;
    // Should still return an error for the file deletion failure
    assert!(result.is_err());

    // But memory state should already be cleaned
    let accounts = manager.accounts.read().await;
    assert!(accounts.is_empty());
    drop(accounts);

    let default_account_id = manager.default_account_id.read().await;
    assert!(default_account_id.is_none());
    drop(default_account_id);

    let api_endpoints = manager.api_endpoints.read().await;
    assert!(api_endpoints.is_empty());
}

#[tokio::test]
async fn test_get_api_endpoint_cache_hit_skips_fetch() {
    // 缓存命中时应直接返回，不发起网络请求
    let temp_dir = tempdir().unwrap();
    let manager = CopilotAuthManager::new(temp_dir.path().to_path_buf());

    let enterprise_endpoint = "https://copilot-api.enterprise.example.com".to_string();
    {
        let mut api_endpoints = manager.api_endpoints.write().await;
        api_endpoints.insert("12345".to_string(), enterprise_endpoint.clone());
    }

    // 即使没有账号数据，缓存命中也应直接返回
    let endpoint = manager.get_api_endpoint("12345").await;
    assert_eq!(endpoint, enterprise_endpoint);
}

#[tokio::test]
async fn test_get_api_endpoint_returns_default_for_unknown_account() {
    let temp_dir = tempdir().unwrap();
    let manager = CopilotAuthManager::new(temp_dir.path().to_path_buf());

    let endpoint = manager.get_api_endpoint("12345").await;
    assert_eq!(endpoint, copilot_api_base(DEFAULT_GITHUB_DOMAIN));
}

#[tokio::test]
async fn test_fetch_and_cache_endpoint_requires_account() {
    // 账号不存在时 fetch_and_cache_endpoint 应返回 AccountNotFound 错误
    let temp_dir = tempdir().unwrap();
    let manager = CopilotAuthManager::new(temp_dir.path().to_path_buf());

    let result = manager.fetch_and_cache_endpoint("nonexistent").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        CopilotAuthError::AccountNotFound(id) => assert_eq!(id, "nonexistent"),
        other => panic!("期望 AccountNotFound 错误，实际: {other:?}"),
    }
}

#[test]
fn test_normalize_github_domain() {
    // 基本用法
    assert_eq!(normalize_github_domain("github.com").unwrap(), "github.com");
    assert_eq!(
        normalize_github_domain("company.ghe.com").unwrap(),
        "company.ghe.com"
    );

    // 剥离协议
    assert_eq!(
        normalize_github_domain("https://company.ghe.com").unwrap(),
        "company.ghe.com"
    );
    assert_eq!(
        normalize_github_domain("http://company.ghe.com").unwrap(),
        "company.ghe.com"
    );

    // 小写化
    assert_eq!(normalize_github_domain("GitHub.COM").unwrap(), "github.com");
    assert_eq!(
        normalize_github_domain("Company.GHE.Com").unwrap(),
        "company.ghe.com"
    );

    // 剥离尾斜杠和 path
    assert_eq!(
        normalize_github_domain("company.ghe.com/").unwrap(),
        "company.ghe.com"
    );
    assert_eq!(
        normalize_github_domain("company.ghe.com/api/v3").unwrap(),
        "company.ghe.com"
    );

    // 剥离 query 和 fragment
    assert_eq!(
        normalize_github_domain("company.ghe.com?foo=bar").unwrap(),
        "company.ghe.com"
    );
    assert_eq!(
        normalize_github_domain("company.ghe.com#section").unwrap(),
        "company.ghe.com"
    );

    // 保留端口
    assert_eq!(
        normalize_github_domain("company.ghe.com:8443").unwrap(),
        "company.ghe.com:8443"
    );

    // 拒绝 userinfo
    assert!(normalize_github_domain("user@company.ghe.com").is_err());

    // 拒绝空输入
    assert!(normalize_github_domain("").is_err());
    assert!(normalize_github_domain("   ").is_err());
}

#[test]
fn test_composite_account_id() {
    // github.com 保持原格式（向后兼容）
    assert_eq!(composite_account_id("github.com", 12345), "12345");

    // GHES 使用复合格式
    assert_eq!(
        composite_account_id("company.ghe.com", 12345),
        "company.ghe.com:12345"
    );

    // 不同 GHES 实例，相同 user ID，不冲突
    assert_ne!(
        composite_account_id("a.ghe.com", 1),
        composite_account_id("b.ghe.com", 1)
    );
}

#[test]
fn test_github_account_from_data_ghes_uses_composite_id() {
    let data = GitHubAccountData {
        github_token: "gho_test".to_string(),
        user: GitHubUser {
            login: "testuser".to_string(),
            id: 99999,
            avatar_url: None,
        },
        authenticated_at: 1700000000,
        github_domain: "company.ghe.com".to_string(),
    };

    let account = GitHubAccount::from(&data);
    assert_eq!(account.id, "company.ghe.com:99999");
}
