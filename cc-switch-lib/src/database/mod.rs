//! Database module - SQLite persistence layer for cc-switch.
//!
//! Organized into domain-specific sub-modules:
//! - `types`    — shared data structures exported from lib.rs
//! - `migrations` — schema creation and migration functions
//! - `providers`  — provider CRUD
//! - `health`     — persisted provider circuit-breaker health
//! - `mcp`        — MCP server CRUD
//! - `skills`     — skill CRUD
//! - `proxy`      — proxy config, live backup
//! - `usage`      — usage records, request logs, aggregates

use crate::config::get_app_config_dir;
use crate::error::AppError;
use rusqlite::Connection;
use std::sync::Mutex;

pub mod backups;
pub mod health;
pub mod mcp;
pub mod migrations;
pub mod providers;
pub mod proxy;
pub mod skills;
pub mod usage;

pub use types::{
    DailyUsage, DataSourceSummary, FailoverQueueItem, LiveBackup, LogFilters, McpServerRecord,
    ModelPricing, ModelStats, PaginatedLogs, Provider, ProviderHealth, ProviderStats,
    ProviderUsageSummary, ProxyConfig, ProxyRequestLogRecord, ProxyType, RequestLogDetail,
    SessionSyncResult, SkillRecord, UsageRecord, UsageSourceItem,
};

mod types;

/// Database connection wrapper
pub struct Database {
    pub(crate) conn: Mutex<Connection>,
}

impl Database {
    #[cfg(test)]
    pub(crate) fn test_memory() -> Self {
        let db = Self {
            conn: Mutex::new(Connection::open_in_memory().unwrap()),
        };
        db.create_tables().unwrap();
        db
    }

    /// Initialize database connection and create tables
    pub fn init() -> Result<Self, AppError> {
        let db_path = get_app_config_dir().join("cc-switch.db");
        Self::init_at(&db_path)
    }

    fn init_at(db_path: &std::path::Path) -> Result<Self, AppError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        let conn = Connection::open(db_path)?;
        conn.execute("PRAGMA foreign_keys = ON;", [])?;
        let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        migrations::ensure_supported_version(&conn)?;
        let has_tables: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table')",
            [],
            |r| r.get(0),
        )?;
        if has_tables && version < migrations::SCHEMA_VERSION {
            backups::before_upgrade(&conn, db_path, version)?;
        }
        if !has_tables {
            conn.execute("PRAGMA auto_vacuum = INCREMENTAL;", [])?;
        }

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.create_tables()?;
        Ok(db)
    }

    /// Create all tables
    fn create_tables(&self) -> Result<(), AppError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("lock failed: {}", e)))?;

        migrations::ensure_supported_version(&conn)?;
        let transaction = conn.unchecked_transaction()?;
        transaction.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS providers (
                id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                name TEXT NOT NULL,
                settings_config TEXT NOT NULL,
                website_url TEXT,
                category TEXT,
                created_at INTEGER,
                sort_index INTEGER,
                notes TEXT,
                icon TEXT,
                icon_color TEXT,
                meta TEXT NOT NULL DEFAULT '{}',
                is_current INTEGER NOT NULL DEFAULT 0,
                in_failover_queue INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (id, app_type)
            );

            CREATE TABLE IF NOT EXISTS provider_endpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                url TEXT NOT NULL,
                added_at INTEGER,
                FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS provider_health (
                provider_id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                circuit_state TEXT NOT NULL DEFAULT 'closed',
                consecutive_failures INTEGER NOT NULL DEFAULT 0,
                last_success_at INTEGER,
                last_failure_at INTEGER,
                updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
                PRIMARY KEY (provider_id, app_type),
                FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS proxy_config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                config_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS proxy_target_config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                active_target_provider_id TEXT
            );

            CREATE TABLE IF NOT EXISTS proxy_port_config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                port INTEGER NOT NULL DEFAULT 15721
            );

            CREATE TABLE IF NOT EXISTS usage_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                model TEXT NOT NULL,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER,
                request_timestamp INTEGER NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            );

            CREATE TABLE IF NOT EXISTS proxy_request_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_type TEXT NOT NULL,
                provider_id TEXT NOT NULL,
                request_path TEXT NOT NULL,
                request_model TEXT,
                status_code INTEGER,
                success INTEGER NOT NULL DEFAULT 0,
                error_message TEXT,
                request_id TEXT,
                model TEXT,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                total_cost_usd TEXT NOT NULL DEFAULT '0',
                data_source TEXT NOT NULL DEFAULT 'proxy',
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            );

            CREATE TABLE IF NOT EXISTS session_log_sync (
                file_path TEXT PRIMARY KEY,
                last_modified INTEGER NOT NULL DEFAULT 0,
                last_line_offset INTEGER NOT NULL DEFAULT 0,
                last_synced_at INTEGER NOT NULL DEFAULT (unixepoch())
            );

            CREATE TABLE IF NOT EXISTS proxy_live_backup (
                app_type TEXT PRIMARY KEY,
                provider_id TEXT NOT NULL,
                original_config TEXT NOT NULL,
                takeover_env TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS mcp_servers (
                id TEXT NOT NULL,
                name TEXT NOT NULL,
                server_spec TEXT NOT NULL,
                app_type TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                PRIMARY KEY (id, app_type)
            );

            CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                directory TEXT NOT NULL,
                app_type TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                collection TEXT,
                installed_at INTEGER NOT NULL DEFAULT 0,
                repo_owner TEXT,
                repo_name TEXT,
                repo_branch TEXT,
                readme_url TEXT
            );

            CREATE TABLE IF NOT EXISTS model_pricing (
                model_id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                input_cost_per_million TEXT NOT NULL DEFAULT '0',
                output_cost_per_million TEXT NOT NULL DEFAULT '0',
                cache_read_cost_per_million TEXT NOT NULL DEFAULT '0',
                cache_creation_cost_per_million TEXT NOT NULL DEFAULT '0'
            );

            CREATE INDEX IF NOT EXISTS idx_usage_provider ON usage_records(provider_id);
            CREATE INDEX IF NOT EXISTS idx_usage_timestamp ON usage_records(request_timestamp);
            CREATE INDEX IF NOT EXISTS idx_usage_request_provider_model
              ON usage_records(request_timestamp, provider_id, model);
            CREATE INDEX IF NOT EXISTS idx_proxy_request_logs_app_type_created_at
              ON proxy_request_logs(app_type, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_proxy_logs_created_provider
              ON proxy_request_logs(created_at, provider_id);
            "
        )?;

        let has_takeover = transaction
            .prepare("PRAGMA table_info(proxy_live_backup)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|name| name == "takeover_env");
        if !has_takeover {
            transaction.execute(
                "ALTER TABLE proxy_live_backup ADD COLUMN takeover_env TEXT",
                [],
            )?;
        }
        migrations::migrate_proxy_config(&transaction)?;
        migrations::migrate_proxy_request_logs(&transaction)?;
        migrations::migrate_session_log_schema(&transaction)?;
        migrations::run_schema_migrations(&transaction)?;

        transaction.commit()?;
        Ok(())
    }

    /// Get database connection lock
    ///
    /// A poisoned lock (from a panic in another thread) still guards a valid
    /// connection, so recover the guard instead of propagating the panic.
    pub(crate) fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_database(directory: &std::path::Path) -> std::path::PathBuf {
        let path = directory.join("cc-switch.db");
        Connection::open(&path)
            .unwrap()
            .execute_batch(include_str!("../../tests/fixtures/v0.2.0.sql"))
            .unwrap();
        path
    }

    #[test]
    fn release_upgrade_preserves_records_and_creates_one_private_original_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let path = legacy_database(directory.path());
        let db = Database::init_at(&path).unwrap();
        assert_eq!(
            db.conn()
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            4
        );
        for table in ["providers", "skills", "mcp_servers"] {
            assert_eq!(
                db.conn()
                    .query_row(
                        &format!("SELECT COUNT(*) FROM {table} WHERE id='legacy'"),
                        [],
                        |r| r.get::<_, u32>(0)
                    )
                    .unwrap(),
                1
            );
        }
        assert!(db
            .conn()
            .query_row(
                "SELECT is_current FROM providers WHERE id='legacy'",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap());
        assert!(!db
            .conn()
            .query_row("SELECT enabled FROM skills WHERE id='legacy'", [], |r| r
                .get::<_, bool>(
                0
            ))
            .unwrap());
        drop(db);
        drop(Database::init_at(&path).unwrap());
        let snapshots: Vec<_> = std::fs::read_dir(directory.path().join("backups/upgrades"))
            .unwrap()
            .map(|r| r.unwrap().path())
            .collect();
        assert_eq!(snapshots.len(), 1);
        let snapshot = Connection::open(&snapshots[0]).unwrap();
        assert_eq!(
            snapshot
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            3
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&snapshots[0])
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        // An old release can resume using the unchanged pre-upgrade snapshot.
        drop(snapshot);
        std::fs::copy(&snapshots[0], &path).unwrap();
        let restored = Connection::open(&path).unwrap();
        assert_eq!(
            restored
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            3
        );
    }

    #[test]
    fn unversioned_wal_database_is_snapshotted_with_committed_wal_records() {
        let directory = tempfile::tempdir().unwrap();
        let path = legacy_database(directory.path());
        let writer = Connection::open(&path).unwrap();
        writer.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; PRAGMA user_version=0; UPDATE providers SET name='Committed WAL name' WHERE id='legacy';").unwrap();
        let db = Database::init_at(&path).unwrap();
        assert_eq!(
            db.conn()
                .query_row("SELECT name FROM providers WHERE id='legacy'", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap(),
            "Committed WAL name"
        );
        let snapshot = std::fs::read_dir(directory.path().join("backups/upgrades"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let backup = Connection::open(snapshot).unwrap();
        assert_eq!(
            backup
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            backup
                .query_row("SELECT name FROM providers WHERE id='legacy'", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap(),
            "Committed WAL name"
        );
    }

    #[test]
    fn backup_failure_and_future_schema_leave_database_untouched() {
        let directory = tempfile::tempdir().unwrap();
        let path = legacy_database(directory.path());
        std::fs::write(directory.path().join("backups"), "block backup directory").unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(Database::init_at(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        Connection::open(&path)
            .unwrap()
            .pragma_update(None, "user_version", 99)
            .unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(Database::init_at(&path)
            .err()
            .unwrap()
            .to_string()
            .contains("newer"));
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn failed_ddl_rolls_back_all_schema_changes_and_retains_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let path = legacy_database(directory.path());
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("DROP INDEX idx_usage_provider; PRAGMA user_version=2; CREATE TRIGGER fail_upgrade BEFORE UPDATE ON proxy_request_logs BEGIN SELECT RAISE(ABORT, 'migration failure'); END; INSERT INTO proxy_request_logs (provider_id, app_type, model, request_path) VALUES ('_session', 'claude_code', 'test-model', '/test');").unwrap();
        let schema: String = connection
            .query_row("SELECT group_concat(sql) FROM sqlite_master", [], |r| {
                r.get(0)
            })
            .unwrap();
        drop(connection);
        assert!(Database::init_at(&path).is_err());
        let connection = Connection::open(&path).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT group_concat(sql) FROM sqlite_master", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap(),
            schema
        );
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            std::fs::read_dir(directory.path().join("backups/upgrades"))
                .unwrap()
                .count(),
            1
        );
    }

    #[test]
    fn version_three_database_gains_durable_deletion_intent_without_replacing_it() {
        let db = Database::test_memory();
        db.conn()
            .execute_batch("DROP TABLE managed_deletion; PRAGMA user_version=3;")
            .unwrap();
        db.create_tables().unwrap();
        let version: u32 = db
            .conn()
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, migrations::SCHEMA_VERSION);
        db.conn()
            .execute(
                "INSERT INTO managed_deletion(id,intent_json) VALUES (1,'retained')",
                [],
            )
            .unwrap();
        db.create_tables().unwrap();
        let value: String = db
            .conn()
            .query_row("SELECT intent_json FROM managed_deletion", [], |r| r.get(0))
            .unwrap();
        assert_eq!(value, "retained");
    }

    #[test]
    fn migrates_legacy_proxy_config_table_to_json_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE proxy_config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                enabled INTEGER NOT NULL,
                proxy_type TEXT NOT NULL,
                host TEXT NOT NULL,
                port INTEGER NOT NULL
            );
            INSERT INTO proxy_config (id, enabled, proxy_type, host, port)
            VALUES (1, 1, 'socks5', '127.0.0.1', 10809);
            ",
        )
        .unwrap();

        migrations::migrate_proxy_config(&conn).unwrap();

        let mut stmt = conn.prepare("PRAGMA table_info(proxy_config)").unwrap();
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(columns.iter().any(|c| c == "config_json"));
        assert!(!columns.iter().any(|c| c == "proxy_type"));

        let config_json: String = conn
            .query_row(
                "SELECT config_json FROM proxy_config WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let config: ProxyConfig = serde_json::from_str(&config_json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.proxy_type, ProxyType::Socks5);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 10809);
    }

    #[test]
    fn schema_migration_does_not_readd_existing_skills_collection_column() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                directory TEXT NOT NULL,
                app_type TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                collection TEXT,
                installed_at INTEGER NOT NULL DEFAULT 0,
                repo_owner TEXT,
                repo_name TEXT,
                repo_branch TEXT,
                readme_url TEXT
            );
            CREATE TABLE proxy_request_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                model TEXT
            );
            PRAGMA user_version = 0;
            ",
        )
        .unwrap();

        migrations::run_schema_migrations(&conn).unwrap();

        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, migrations::SCHEMA_VERSION);
    }
    #[test]
    fn legacy_route_backup_gains_optional_takeover_snapshot() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE proxy_live_backup (app_type TEXT PRIMARY KEY, provider_id TEXT NOT NULL, original_config TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now'))); INSERT INTO proxy_live_backup (app_type, provider_id, original_config) VALUES ('claude_code', 'route', '{}');").unwrap();
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.create_tables().unwrap();
        db.create_tables().unwrap();
        let backup = db.get_live_backup("claude_code").unwrap().unwrap();
        assert!(backup.takeover_env.is_none());
        assert_eq!(backup.original_config, "{}");
        let env = serde_json::json!({"CLAUDE_CODE_AUTO_MODE_SERVER": "0"})
            .as_object()
            .unwrap()
            .clone();
        db.save_live_backup_with_takeover("claude_code", "route", "{}", Some(&env))
            .unwrap();
        assert_eq!(
            db.get_live_backup("claude_code")
                .unwrap()
                .unwrap()
                .takeover_env,
            Some(env)
        );
    }
}
