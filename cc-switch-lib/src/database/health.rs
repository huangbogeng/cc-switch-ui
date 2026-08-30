//! Provider circuit-breaker health persistence.

use crate::database::types::ProviderHealth;
use crate::error::AppError;
use rusqlite::params;

impl super::Database {
    pub fn record_provider_health(
        &self,
        app_type: &str,
        provider_id: &str,
        circuit_state: &str,
        consecutive_failures: u32,
        succeeded: bool,
    ) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO provider_health (
                provider_id, app_type, circuit_state, consecutive_failures,
                last_success_at, last_failure_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4,
                CASE WHEN ?5 THEN unixepoch() END,
                CASE WHEN ?5 THEN NULL ELSE unixepoch() END,
                unixepoch()
             )
             ON CONFLICT(provider_id, app_type) DO UPDATE SET
                circuit_state = excluded.circuit_state,
                consecutive_failures = excluded.consecutive_failures,
                last_success_at = CASE
                    WHEN ?5 THEN unixepoch() ELSE provider_health.last_success_at END,
                last_failure_at = CASE
                    WHEN ?5 THEN provider_health.last_failure_at ELSE unixepoch() END,
                updated_at = unixepoch()",
            params![
                provider_id,
                app_type,
                circuit_state,
                consecutive_failures,
                succeeded,
            ],
        )?;
        Ok(())
    }

    pub fn list_provider_health(&self, app_type: &str) -> Result<Vec<ProviderHealth>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT provider_id, app_type, circuit_state, consecutive_failures,
                    last_success_at, last_failure_at, updated_at
             FROM provider_health WHERE app_type = ?1 ORDER BY provider_id",
        )?;
        let rows = stmt.query_map(params![app_type], |row| {
            Ok(ProviderHealth {
                provider_id: row.get(0)?,
                app_type: row.get(1)?,
                circuit_state: row.get(2)?,
                consecutive_failures: row.get(3)?,
                last_success_at: row.get(4)?,
                last_failure_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::super::Database;
    use rusqlite::Connection;
    use std::sync::Mutex;

    fn database() -> Database {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE provider_health (
                provider_id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                circuit_state TEXT NOT NULL,
                consecutive_failures INTEGER NOT NULL,
                last_success_at INTEGER,
                last_failure_at INTEGER,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (provider_id, app_type)
            );",
        )
        .unwrap();
        Database {
            conn: Mutex::new(conn),
        }
    }

    #[test]
    fn health_snapshot_tracks_failure_then_recovery() {
        let db = database();
        db.record_provider_health("claude_code", "provider-1", "open", 3, false)
            .unwrap();

        let failed = db.list_provider_health("claude_code").unwrap();
        assert_eq!(failed[0].circuit_state, "open");
        assert_eq!(failed[0].consecutive_failures, 3);
        assert!(failed[0].last_failure_at.is_some());

        db.record_provider_health("claude_code", "provider-1", "closed", 0, true)
            .unwrap();
        let recovered = db.list_provider_health("claude_code").unwrap();
        assert_eq!(recovered[0].circuit_state, "closed");
        assert_eq!(recovered[0].consecutive_failures, 0);
        assert!(recovered[0].last_success_at.is_some());
        assert!(recovered[0].last_failure_at.is_some());
    }
}
