//! MCP server CRUD operations.

use crate::database::types::McpServerRecord;
use crate::error::AppError;
use rusqlite::params;

impl super::Database {
    pub fn get_all_mcp_servers(&self, app_type: &str) -> Result<Vec<McpServerRecord>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, server_spec, app_type, enabled
             FROM mcp_servers WHERE app_type = ?1",
        )?;
        let rows = stmt.query_map(params![app_type], |row| {
            let spec_str: String = row.get(2)?;
            Ok(McpServerRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                server_spec: serde_json::from_str(&spec_str).unwrap_or(serde_json::Value::Null),
                app_type: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(result)
    }

    pub fn get_enabled_mcp_servers(&self, app_type: &str) -> Result<Vec<McpServerRecord>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, server_spec, app_type, enabled
             FROM mcp_servers WHERE app_type = ?1 AND enabled = 1",
        )?;
        let rows = stmt.query_map(params![app_type], |row| {
            let spec_str: String = row.get(2)?;
            Ok(McpServerRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                server_spec: serde_json::from_str(&spec_str).unwrap_or(serde_json::Value::Null),
                app_type: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(result)
    }

    pub fn save_mcp_server(&self, server: &McpServerRecord) -> Result<(), AppError> {
        let conn = self.conn();
        let spec_str = serde_json::to_string(&server.server_spec)
            .map_err(|e| AppError::JsonSerialize { source: e })?;
        conn.execute(
            "INSERT INTO mcp_servers (id, name, server_spec, app_type, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id, app_type) DO UPDATE SET
                name = excluded.name,
                server_spec = excluded.server_spec,
                enabled = excluded.enabled",
            params![
                server.id,
                server.name,
                spec_str,
                server.app_type,
                server.enabled as i32,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn delete_mcp_server(&self, id: &str, app_type: &str) -> Result<bool, AppError> {
        let conn = self.conn();
        let affected = conn
            .execute(
                "DELETE FROM mcp_servers WHERE id = ?1 AND app_type = ?2",
                params![id, app_type],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(affected > 0)
    }
}
