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
    /// Initialize database connection and create tables
    pub fn init() -> Result<Self, AppError> {
        let db_path = get_app_config_dir().join("cc-switch.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        let conn = Connection::open(&db_path)?;
        conn.execute("PRAGMA foreign_keys = ON;", [])?;
        conn.execute("PRAGMA auto_vacuum = INCREMENTAL;", [])?;

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

        conn.execute_batch(
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

        migrations::migrate_proxy_config(&conn)?;
        migrations::migrate_proxy_request_logs(&conn)?;
        migrations::migrate_session_log_schema(&conn)?;
        migrations::run_schema_migrations(&conn)?;

        Ok(())
    }

    /// Get database connection lock
    pub(crate) fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("database lock poisoned")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(version, 3);
    }
}
