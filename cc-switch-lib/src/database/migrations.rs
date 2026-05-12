//! Database schema migrations.

use crate::database::types::{ProxyConfig, ProxyType};
use crate::error::AppError;
use rusqlite::Connection;

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
            rusqlite::params![config_json],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    Ok(())
}

fn migrate_proxy_request_logs_schema(conn: &Connection) -> Result<(), AppError> {
    let columns = table_columns(conn, "proxy_request_logs")?;
    if columns.is_empty() {
        return Ok(());
    }

    let has_column = |name: &str| columns.iter().any(|column| column == name);

    if !has_column("request_path") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN request_path TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    if !has_column("request_model") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN request_model TEXT",
            [],
        )?;
    }
    if !has_column("status_code") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN status_code INTEGER",
            [],
        )?;
    }
    if !has_column("success") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN success INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !has_column("error_message") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN error_message TEXT",
            [],
        )?;
    }
    if !has_column("request_id") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN request_id TEXT",
            [],
        )?;
    }
    if !has_column("model") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN model TEXT",
            [],
        )?;
    }
    if !has_column("input_tokens") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN input_tokens INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !has_column("output_tokens") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN output_tokens INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !has_column("cache_read_tokens") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN cache_read_tokens INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !has_column("cache_creation_tokens") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN cache_creation_tokens INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !has_column("total_cost_usd") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN total_cost_usd TEXT NOT NULL DEFAULT '0'",
            [],
        )?;
    }
    if !has_column("data_source") {
        conn.execute(
            "ALTER TABLE proxy_request_logs ADD COLUMN data_source TEXT NOT NULL DEFAULT 'proxy'",
            [],
        )?;
    }

    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_proxy_request_logs_request_id ON proxy_request_logs(request_id);
         CREATE INDEX IF NOT EXISTS idx_proxy_request_logs_data_source ON proxy_request_logs(data_source);"
    )?;

    Ok(())
}

pub fn migrate_session_log_schema(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS session_log_sync (
            file_path TEXT PRIMARY KEY,
            last_modified INTEGER NOT NULL DEFAULT 0,
            last_line_offset INTEGER NOT NULL DEFAULT 0,
            last_synced_at INTEGER NOT NULL DEFAULT (unixepoch())
        );"
    )?;
    Ok(())
}

/// Current schema version for PRAGMA user_version-based migration tracking.
const SCHEMA_VERSION: u32 = 3;

pub fn run_schema_migrations(conn: &Connection) -> Result<(), AppError> {
    let current_version: u32 =
        conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if current_version >= SCHEMA_VERSION {
        return Ok(());
    }

    if current_version < 2 {
        let has_column: bool = conn
            .pragma_query_value(None, "table_info(skills)", |row| {
                let col_name: String = row.get(1)?;
                Ok(col_name == "collection")
            })
            .unwrap_or(false);
        if !has_column {
            conn.execute_batch("ALTER TABLE skills ADD COLUMN collection TEXT")?;
        }
    }

    if current_version < 3 {
        // Migrate legacy _session provider_ids to model-derived names
        conn.execute_batch(
            "UPDATE proxy_request_logs
             SET provider_id = LOWER(SUBSTR(model, 1, INSTR(model || '-', '-') - 1))
             WHERE provider_id = '_session' AND model != '';"
        )?;
    }

    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

// ---- legacy config reader ----

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

pub fn migrate_proxy_config(conn: &Connection) -> Result<(), AppError> {
    migrate_proxy_config_schema(conn)
}

pub fn migrate_proxy_request_logs(conn: &Connection) -> Result<(), AppError> {
    migrate_proxy_request_logs_schema(conn)
}
