//! Provider CRUD operations.

use crate::database::types::Provider;
use crate::error::AppError;
use rusqlite::params;
use std::collections::HashMap;

impl super::Database {
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

    pub fn delete_provider(&self, id: &str, app_type: &str) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM providers WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

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
}
