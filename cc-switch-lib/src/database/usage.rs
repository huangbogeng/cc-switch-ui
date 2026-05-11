//! Usage tracking and request log operations.

use crate::database::types::{
    DailyUsage, ProviderUsageSummary, ProxyRequestLogEntry, ProxyRequestLogRecord, UsageRecord,
};
use crate::error::AppError;
use rusqlite::params;

impl super::Database {
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

    pub fn get_proxy_request_logs(&self, limit: usize) -> Result<Vec<ProxyRequestLogEntry>, AppError> {
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
