//! Proxy and live backup operations.

use crate::database::types::{LiveBackup, ProxyConfig};
use crate::error::AppError;
use rusqlite::params;

impl super::Database {
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

    pub fn delete_proxy_config(&self) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute("DELETE FROM proxy_config WHERE id = 1", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

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
