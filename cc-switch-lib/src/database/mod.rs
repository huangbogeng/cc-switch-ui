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
        let conn = self.conn.lock()
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
                id INTEGER PRIMARY KEY,
                config_json TEXT NOT NULL
            );
            "
        )?;
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
             FROM providers WHERE app_type = ?1 ORDER BY sort_index ASC NULLS LAST"
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
                meta: serde_json::from_str(&meta_str).unwrap_or(serde_json::Value::Object(Default::default())),
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
             FROM providers WHERE id = ?1 AND app_type = ?2"
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
                meta: serde_json::from_str(&meta_str).unwrap_or(serde_json::Value::Object(Default::default())),
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
        let meta_str = serde_json::to_string(&provider.meta)
            .unwrap_or_else(|_| "{}".to_string());

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
        ).map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete a provider
    pub fn delete_provider(&self, id: &str, app_type: &str) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM providers WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        ).map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Set current provider for an app (clear other is_current flags, set this one)
    pub fn set_current_provider(&self, id: &str, app_type: &str) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "UPDATE providers SET is_current = 0 WHERE app_type = ?1",
            params![app_type],
        ).map_err(|e| AppError::Database(e.to_string()))?;
        conn.execute(
            "UPDATE providers SET is_current = 1 WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        ).map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get current provider ID for an app
    pub fn get_current_provider_id(&self, app_type: &str) -> Result<Option<String>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id FROM providers WHERE app_type = ?1 AND is_current = 1"
        )?;
        let mut rows = stmt.query(params![app_type])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }
}

/// Failover queue item for external reference
pub struct FailoverQueueItem;
