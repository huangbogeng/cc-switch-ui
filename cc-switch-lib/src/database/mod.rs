//! Database module - Phase 1 minimal implementation
//!
//! Provides SQLite persistence for providers and proxy config.

use crate::config::get_app_config_dir;
use crate::error::AppError;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Provider struct matching frontend Provider interface
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub settings_config: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_color: Option<String>,
    #[serde(default)]
    pub meta: serde_json::Value,
    #[serde(default)]
    pub in_failover_queue: bool,
}

/// Proxy configuration for OAuth authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub enabled: bool,
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub auto_failover_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyType {
    Http,
    Socks5,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            proxy_type: ProxyType::Http,
            host: String::new(),
            port: 10809,
            auto_failover_enabled: false,
        }
    }
}

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
    pub(crate) fn create_tables(&self) -> Result<(), AppError> {
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
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            );

            CREATE TABLE IF NOT EXISTS proxy_live_backup (
                app_type TEXT PRIMARY KEY,
                provider_id TEXT NOT NULL,
                original_config TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_usage_provider ON usage_records(provider_id);
            CREATE INDEX IF NOT EXISTS idx_usage_timestamp ON usage_records(request_timestamp);
            CREATE INDEX IF NOT EXISTS idx_proxy_request_logs_app_type_created_at
              ON proxy_request_logs(app_type, created_at DESC);
            "
        )?;
        migrate_proxy_config_schema(&conn)?;
        migrate_proxy_request_logs_schema(&conn)?;
        run_schema_migrations(&conn)?;
        Ok(())
    }

    /// Get database connection lock
    pub(crate) fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("database lock poisoned")
    }

    /// List all providers for an app, returns Record<id, Provider>
    pub fn list_providers(&self, app_type: &str) -> Result<HashMap<String, Provider>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, settings_config, website_url, category, created_at,
                    sort_index, notes, icon, icon_color, meta, in_failover_queue
             FROM providers WHERE app_type = ?1 ORDER BY sort_index ASC NULLS LAST",
        )?;

        let rows = stmt.query_map(params![app_type], |row| {
            let settings_config_str: String = row.get(2)?;
            let meta_str: String = row.get(10)?;
            Ok(Provider {
                id: row.get(0)?,
                name: row.get(1)?,
                settings_config: serde_json::from_str(&settings_config_str)
                    .unwrap_or(serde_json::Value::Null),
                website_url: row.get(3)?,
                category: row.get(4)?,
                created_at: row.get(5)?,
                sort_index: row.get(6)?,
                notes: row.get(7)?,
                icon: row.get(8)?,
                icon_color: row.get(9)?,
                meta: serde_json::from_str(&meta_str)
                    .unwrap_or(serde_json::Value::Object(Default::default())),
                in_failover_queue: row.get::<_, i32>(11)? != 0,
            })
        })?;

        let mut map = HashMap::new();
        for provider in rows {
            let p = provider.map_err(|e| AppError::Database(e.to_string()))?;
            map.insert(p.id.clone(), p);
        }
        Ok(map)
    }

    /// Get a single provider by id and app_type
    pub fn get_provider(&self, id: &str, app_type: &str) -> Result<Option<Provider>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, settings_config, website_url, category, created_at,
                    sort_index, notes, icon, icon_color, meta, in_failover_queue
             FROM providers WHERE id = ?1 AND app_type = ?2",
        )?;

        let mut rows = stmt.query_map(params![id, app_type], |row| {
            let settings_config_str: String = row.get(2)?;
            let meta_str: String = row.get(10)?;
            Ok(Provider {
                id: row.get(0)?,
                name: row.get(1)?,
                settings_config: serde_json::from_str(&settings_config_str)
                    .unwrap_or(serde_json::Value::Null),
                website_url: row.get(3)?,
                category: row.get(4)?,
                created_at: row.get(5)?,
                sort_index: row.get(6)?,
                notes: row.get(7)?,
                icon: row.get(8)?,
                icon_color: row.get(9)?,
                meta: serde_json::from_str(&meta_str)
                    .unwrap_or(serde_json::Value::Object(Default::default())),
                in_failover_queue: row.get::<_, i32>(11)? != 0,
            })
        })?;

        match rows.next() {
            Some(r) => Ok(Some(r.map_err(|e| AppError::Database(e.to_string()))?)),
            None => Ok(None),
        }
    }

    /// Save (upsert) a provider
    pub fn save_provider(&self, app_type: &str, provider: &Provider) -> Result<(), AppError> {
        let conn = self.conn();
        let settings_config_str = serde_json::to_string(&provider.settings_config)
            .map_err(|e| AppError::JsonSerialize { source: e })?;
        let meta_str = serde_json::to_string(&provider.meta).unwrap_or_else(|_| "{}".to_string());

        conn.execute(
            "INSERT INTO providers (id, app_type, name, settings_config, website_url, category,
                created_at, sort_index, notes, icon, icon_color, meta, in_failover_queue)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id, app_type) DO UPDATE SET
                name = excluded.name,
                settings_config = excluded.settings_config,
                website_url = excluded.website_url,
                category = excluded.category,
                sort_index = excluded.sort_index,
                notes = excluded.notes,
                icon = excluded.icon,
                icon_color = excluded.icon_color,
                meta = excluded.meta,
                in_failover_queue = excluded.in_failover_queue",
            params![
                provider.id,
                app_type,
                provider.name,
                settings_config_str,
                provider.website_url,
                provider.category,
                provider.created_at,
                provider.sort_index,
                provider.notes,
                provider.icon,
                provider.icon_color,
                meta_str,
                provider.in_failover_queue as i32,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete a provider
    pub fn delete_provider(&self, id: &str, app_type: &str) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM providers WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Set current provider for an app (clear other is_current flags, set this one)
    pub fn set_current_provider(&self, id: &str, app_type: &str) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "UPDATE providers SET is_current = 0 WHERE app_type = ?1",
            params![app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        conn.execute(
            "UPDATE providers SET is_current = 1 WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get current provider ID for an app
    pub fn get_current_provider_id(&self, app_type: &str) -> Result<Option<String>, AppError> {
        let conn = self.conn();
        let mut stmt =
            conn.prepare("SELECT id FROM providers WHERE app_type = ?1 AND is_current = 1")?;
        let mut rows = stmt.query(params![app_type])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    /// Get the active provider target for the local proxy.
    pub fn get_proxy_target_provider_id(&self) -> Result<Option<String>, AppError> {
        let conn = self.conn();
        let result: Result<Option<String>, rusqlite::Error> = conn.query_row(
            "SELECT active_target_provider_id FROM proxy_target_config WHERE id = 1",
            [],
            |row| row.get(0),
        );

        match result {
            Ok(value) => Ok(value),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    /// Set the active provider target for the local proxy.
    pub fn set_proxy_target_provider_id(&self, provider_id: &str) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO proxy_target_config (id, active_target_provider_id) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET active_target_provider_id = excluded.active_target_provider_id",
            params![provider_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get proxy configuration
    pub fn get_proxy_config(&self) -> Result<Option<ProxyConfig>, AppError> {
        let conn = self.conn();
        let result: Result<String, rusqlite::Error> = conn.query_row(
            "SELECT config_json FROM proxy_config WHERE id = 1",
            [],
            |row| row.get(0),
        );

        match result {
            Ok(json_str) => {
                let config: ProxyConfig = serde_json::from_str(&json_str)
                    .map_err(|e| AppError::Database(format!("Invalid proxy config JSON: {}", e)))?;
                Ok(Some(config))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    /// Set proxy configuration
    pub fn set_proxy_config(&self, config: &ProxyConfig) -> Result<(), AppError> {
        let conn = self.conn();
        let config_json =
            serde_json::to_string(config).map_err(|e| AppError::JsonSerialize { source: e })?;

        conn.execute(
            "INSERT INTO proxy_config (id, config_json) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET config_json = excluded.config_json",
            params![config_json],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete proxy configuration
    pub fn delete_proxy_config(&self) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute("DELETE FROM proxy_config WHERE id = 1", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get proxy listen port
    pub fn get_proxy_port(&self) -> Result<u16, AppError> {
        let conn = self.conn();
        let result: Result<i64, rusqlite::Error> = conn.query_row(
            "SELECT port FROM proxy_port_config WHERE id = 1",
            [],
            |row| row.get(0),
        );

        match result {
            Ok(port) => Ok(port as u16),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(15721),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    /// Set proxy listen port
    pub fn set_proxy_port(&self, port: u16) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO proxy_port_config (id, port) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET port = excluded.port",
            params![port as i64],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get live backup record for an app type.
    pub fn get_live_backup(&self, app_type: &str) -> Result<Option<LiveBackup>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT app_type, provider_id, original_config, created_at
             FROM proxy_live_backup WHERE app_type = ?1",
        )?;
        let mut rows = stmt.query_map(params![app_type], |row| {
            Ok(LiveBackup {
                app_type: row.get(0)?,
                provider_id: row.get(1)?,
                original_config: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r.map_err(|e| AppError::Database(e.to_string()))?)),
            None => Ok(None),
        }
    }

    /// Save or update live backup record.
    pub fn save_live_backup(
        &self,
        app_type: &str,
        provider_id: &str,
        original_config: &str,
    ) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO proxy_live_backup (app_type, provider_id, original_config)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(app_type) DO UPDATE SET
                provider_id = excluded.provider_id,
                original_config = excluded.original_config,
                created_at = datetime('now')",
            params![app_type, provider_id, original_config],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete live backup record.
    pub fn delete_live_backup(&self, app_type: &str) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM proxy_live_backup WHERE app_type = ?1",
            params![app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}

/// Usage record for database storage
#[derive(Debug, Clone)]
pub struct UsageRecord {
    pub provider_id: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: Option<i64>,
    pub request_timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct ProxyRequestLogRecord {
    pub app_type: String,
    pub provider_id: String,
    pub request_path: String,
    pub request_model: Option<String>,
    pub status_code: Option<i32>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProxyRequestLogEntry {
    pub app_type: String,
    pub provider_id: String,
    pub request_path: String,
    pub request_model: Option<String>,
    pub status_code: Option<i32>,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: i64,
}

/// Live backup record for proxy takeover detection/restore
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveBackup {
    pub app_type: String,
    pub provider_id: String,
    pub original_config: String,
    pub created_at: String,
}

impl Database {
    /// Save a usage record
    pub fn save_usage_record(&self, record: &UsageRecord) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO usage_records (provider_id, model, input_tokens, output_tokens, cache_read_tokens, request_timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                record.provider_id,
                record.model,
                record.input_tokens,
                record.output_tokens,
                record.cache_read_tokens,
                record.request_timestamp,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn save_proxy_request_log(&self, record: &ProxyRequestLogRecord) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO proxy_request_logs (
                app_type, provider_id, request_path, request_model,
                status_code, success, error_message
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.app_type,
                record.provider_id,
                record.request_path,
                record.request_model,
                record.status_code,
                if record.success { 1 } else { 0 },
                record.error_message,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_proxy_request_logs(
        &self,
        limit: usize,
    ) -> Result<Vec<ProxyRequestLogEntry>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT app_type, provider_id, request_path, request_model,
                    status_code, success, error_message, created_at
             FROM proxy_request_logs
             ORDER BY created_at DESC, id DESC
             LIMIT ?1",
        )?;
        let mut rows = stmt.query(params![limit as i64])?;
        let mut entries = Vec::new();
        while let Some(row) = rows.next()? {
            entries.push(ProxyRequestLogEntry {
                app_type: row.get(0)?,
                provider_id: row.get(1)?,
                request_path: row.get(2)?,
                request_model: row.get(3)?,
                status_code: row.get(4)?,
                success: row.get::<_, i64>(5)? != 0,
                error_message: row.get(6)?,
                created_at: row.get(7)?,
            });
        }
        Ok(entries)
    }

    /// Get usage summary by provider
    pub fn get_usage_summary_by_provider(
        &self,
        since_timestamp: Option<i64>,
    ) -> Result<Vec<ProviderUsageSummary>, AppError> {
        let conn = self.conn();
        let where_clause = since_timestamp
            .map(|_| "WHERE request_timestamp >= ?1")
            .unwrap_or("");

        let sql = format!(
            "SELECT provider_id, model, SUM(input_tokens) as total_input,
                    SUM(output_tokens) as total_output, COUNT(*) as request_count
             FROM usage_records
             {}
             GROUP BY provider_id, model
             ORDER BY total_input + total_output DESC",
            where_clause
        );

        let mut stmt = conn.prepare(&sql)?;
        let mut rows = if let Some(ts) = since_timestamp {
            stmt.query(params![ts])?
        } else {
            stmt.query([])?
        };

        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            results.push(ProviderUsageSummary {
                provider_id: row.get(0)?,
                model: row.get(1)?,
                total_input_tokens: row.get(2)?,
                total_output_tokens: row.get(3)?,
                request_count: row.get(4)?,
            });
        }
        Ok(results)
    }

    /// Get usage trend (daily aggregates)
    pub fn get_usage_daily_trend(&self, days: i32) -> Result<Vec<DailyUsage>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT date(request_timestamp, 'unixepoch') as day,
                    SUM(input_tokens) as total_input, SUM(output_tokens) as total_output,
                    COUNT(*) as request_count
             FROM usage_records
             WHERE request_timestamp >= ?1
             GROUP BY day
             ORDER BY day DESC",
        )?;

        let cutoff = chrono::Utc::now().timestamp() - (days as i64 * 86400);
        let mut rows = stmt.query(params![cutoff])?;

        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            results.push(DailyUsage {
                day: row.get(0)?,
                total_input_tokens: row.get(1)?,
                total_output_tokens: row.get(2)?,
                request_count: row.get(3)?,
            });
        }
        Ok(results)
    }
}

fn migrate_proxy_config_schema(conn: &Connection) -> Result<(), AppError> {
    let columns = table_columns(conn, "proxy_config")?;
    if columns.is_empty() || columns.iter().any(|column| column == "config_json") {
        return Ok(());
    }

    let legacy_config = read_legacy_proxy_config(conn, &columns)?;

    conn.execute("DROP TABLE proxy_config", [])
        .map_err(|e| AppError::Database(e.to_string()))?;
    conn.execute(
        "CREATE TABLE proxy_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            config_json TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    if let Some(config) = legacy_config {
        let config_json =
            serde_json::to_string(&config).map_err(|e| AppError::JsonSerialize { source: e })?;
        conn.execute(
            "INSERT INTO proxy_config (id, config_json) VALUES (1, ?1)",
            params![config_json],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    Ok(())
}

fn migrate_proxy_request_logs_schema(conn: &Connection) -> Result<(), AppError> {
    let columns = table_columns(conn, "proxy_request_logs")?;
    if columns.is_empty() {
        // Table does not exist yet (fresh DB path); CREATE TABLE handles it.
        return Ok(());
    }

    let has_column = |name: &str| columns.iter().any(|column| column == name);

    if !has_column("request_path") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN request_path TEXT NOT NULL DEFAULT ''",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    if !has_column("request_model") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN request_model TEXT",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    if !has_column("status_code") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN status_code INTEGER",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    if !has_column("success") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN success INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    if !has_column("error_message") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN error_message TEXT",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    Ok(())
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>, AppError> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| AppError::Database(e.to_string()))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut columns = Vec::new();
    for row in rows {
        columns.push(row.map_err(|e| AppError::Database(e.to_string()))?);
    }
    Ok(columns)
}

fn read_legacy_proxy_config(
    conn: &Connection,
    columns: &[String],
) -> Result<Option<ProxyConfig>, AppError> {
    let has_column = |name: &str| columns.iter().any(|column| column == name);
    if !(has_column("enabled")
        && has_column("proxy_type")
        && has_column("host")
        && has_column("port"))
    {
        log::warn!(
            "[Database] Dropping unsupported legacy proxy_config schema: columns={:?}",
            columns
        );
        return Ok(None);
    }

    let sql = if has_column("id") {
        "SELECT enabled, proxy_type, host, port FROM proxy_config WHERE id = 1"
    } else {
        "SELECT enabled, proxy_type, host, port FROM proxy_config LIMIT 1"
    };
    let result: Result<ProxyConfig, rusqlite::Error> = conn.query_row(sql, [], |row| {
        let enabled = row.get::<_, i64>(0)? != 0;
        let proxy_type_raw: String = row.get(1)?;
        let proxy_type = match proxy_type_raw.to_ascii_lowercase().as_str() {
            "socks" | "socks5" => ProxyType::Socks5,
            _ => ProxyType::Http,
        };
        let host = row.get(2)?;
        let port = row.get::<_, i64>(3)? as u16;
        Ok(ProxyConfig {
            enabled,
            proxy_type,
            host,
            port,
            auto_failover_enabled: false,
        })
    });

    match result {
        Ok(config) => Ok(Some(config)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Database(e.to_string())),
    }
}

/// Current schema version for PRAGMA user_version-based migration tracking.
const SCHEMA_VERSION: u32 = 1;

/// Run schema migrations based on PRAGMA user_version.
///
/// New migrations should be added as conditional blocks:
/// ```ignore
/// if current_version < 2 {
///     // v2 migration steps
/// }
/// ```
fn run_schema_migrations(conn: &Connection) -> Result<(), AppError> {
    let current_version: u32 =
        conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if current_version >= SCHEMA_VERSION {
        return Ok(());
    }

    // v1: proxy_live_backup table — created in create_tables() with IF NOT EXISTS,
    // so existing databases pick it up on next startup automatically.
    // No additional migration steps needed.

    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

/// Usage summary by provider
#[derive(Debug, Clone, Serialize)]
pub struct ProviderUsageSummary {
    pub provider_id: String,
    pub model: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub request_count: i64,
}

/// Daily usage aggregate
#[derive(Debug, Clone, Serialize)]
pub struct DailyUsage {
    pub day: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub request_count: i64,
}

/// Failover queue item for external reference
pub struct FailoverQueueItem;

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

        migrate_proxy_config_schema(&conn).unwrap();

        let columns = table_columns(&conn, "proxy_config").unwrap();
        assert!(columns.iter().any(|column| column == "config_json"));
        assert!(!columns.iter().any(|column| column == "proxy_type"));

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
}
