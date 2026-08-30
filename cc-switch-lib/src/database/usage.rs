//! Usage tracking and request log operations.

use crate::database::types::{
    DailyUsage, LogFilters, ModelPricing, ModelStats, PaginatedLogs, ProviderStats,
    ProviderUsageSummary, ProxyRequestLogRecord, RequestLogDetail, UsageRecord, UsageSourceItem,
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
                status_code, success, error_message,
                request_id, model, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens, total_cost_usd, data_source
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                record.app_type,
                record.provider_id,
                record.request_path,
                record.request_model,
                record.status_code,
                if record.success { 1 } else { 0 },
                record.error_message,
                record.request_id,
                record.model,
                record.input_tokens,
                record.output_tokens,
                record.cache_read_tokens,
                record.cache_creation_tokens,
                record.total_cost_usd,
                record.data_source,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_usage_summary_by_provider(
        &self,
        start_date: Option<i64>,
        end_date: Option<i64>,
    ) -> Result<Vec<ProviderUsageSummary>, AppError> {
        let conn = self.conn();
        let (where_clause, params_list) = build_ts_where("created_at", start_date, end_date);

        let sql = format!(
            "SELECT provider_id, model, SUM(input_tokens) as total_input,
                    SUM(output_tokens) as total_output, COUNT(*) as request_count
             FROM proxy_request_logs
             {}
             GROUP BY provider_id, model
             ORDER BY total_input + total_output DESC",
            where_clause
        );

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_list.iter().map(|param| param.as_ref()).collect();
        let mut rows = stmt.query(param_refs.as_slice())?;

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

    pub fn get_usage_daily_trend(
        &self,
        start_date: Option<i64>,
        end_date: Option<i64>,
    ) -> Result<Vec<DailyUsage>, AppError> {
        let conn = self.conn();
        let (where_clause, params_list) = build_ts_where("created_at", start_date, end_date);
        let sql = format!(
            "SELECT date(created_at, 'unixepoch') as day,
                    SUM(input_tokens) as total_input, SUM(output_tokens) as total_output,
                    COUNT(*) as request_count
             FROM proxy_request_logs
             {}
             GROUP BY day
             ORDER BY day DESC",
            where_clause
        );
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_list.iter().map(|param| param.as_ref()).collect();
        let mut rows = stmt.query(param_refs.as_slice())?;

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

    /// Aggregate provider stats from proxy_request_logs.
    pub fn get_usage_provider_stats(
        &self,
        start_date: Option<i64>,
        end_date: Option<i64>,
    ) -> Result<Vec<ProviderStats>, AppError> {
        log::info!(
            "[DB] get_usage_provider_stats start={:?} end={:?}",
            start_date,
            end_date
        );
        let _start = std::time::Instant::now();
        let results = {
            let conn = self.conn();
            let (where_clause, params_list) = build_ts_where("created_at", start_date, end_date);
            let sql = format!(
                "SELECT provider_id,
                        COUNT(*) as request_count,
                        COALESCE(SUM(input_tokens), 0) as total_input,
                        COALESCE(SUM(output_tokens), 0) as total_output,
                        SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                        SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as fail_count
                 FROM proxy_request_logs
                 {}
                 GROUP BY provider_id
                 ORDER BY request_count DESC",
                where_clause
            );

            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                params_list.iter().map(|p| p.as_ref()).collect();
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query(param_refs.as_slice())?;

            let mut results = Vec::new();
            while let Some(row) = rows.next()? {
                results.push(ProviderStats {
                    provider_id: row.get(0)?,
                    request_count: row.get(1)?,
                    total_input_tokens: row.get(2)?,
                    total_output_tokens: row.get(3)?,
                    success_count: row.get(4)?,
                    fail_count: row.get(5)?,
                });
            }
            results
        }; // conn dropped here, lock released

        // Fallback to usage_records if proxy_request_logs is empty
        if results.is_empty() {
            log::info!(
                "[DB] proxy_request_logs empty, falling back to usage_records ({}ms)",
                _start.elapsed().as_millis()
            );
            return self.get_usage_provider_stats_fallback(start_date, end_date);
        }

        log::info!(
            "[DB] get_usage_provider_stats done: {} providers ({}ms)",
            results.len(),
            _start.elapsed().as_millis()
        );
        Ok(results)
    }

    /// Fallback: aggregate from usage_records (for backward compat during migration)
    fn get_usage_provider_stats_fallback(
        &self,
        start_date: Option<i64>,
        end_date: Option<i64>,
    ) -> Result<Vec<ProviderStats>, AppError> {
        let _start = std::time::Instant::now();
        log::info!(
            "[DB] get_usage_provider_stats_fallback start={:?} end={:?}",
            start_date,
            end_date
        );
        let conn = self.conn();
        let (where_clause, params_list) = build_ts_where("request_timestamp", start_date, end_date);
        let sql = format!(
            "SELECT provider_id,
                    COUNT(*) as request_count,
                    SUM(input_tokens) as total_input,
                    SUM(output_tokens) as total_output
             FROM usage_records
             {}
             GROUP BY provider_id
             ORDER BY request_count DESC",
            where_clause
        );
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_list.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query(param_refs.as_slice())?;
        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            results.push(ProviderStats {
                provider_id: row.get(0)?,
                request_count: row.get(1)?,
                total_input_tokens: row.get(2)?,
                total_output_tokens: row.get(3)?,
                success_count: 0,
                fail_count: 0,
            });
        }
        log::info!(
            "[DB] get_usage_provider_stats_fallback done: {} providers ({}ms)",
            results.len(),
            _start.elapsed().as_millis()
        );
        Ok(results)
    }

    /// Aggregate model stats from proxy_request_logs.
    pub fn get_usage_model_stats(
        &self,
        start_date: Option<i64>,
        end_date: Option<i64>,
    ) -> Result<Vec<ModelStats>, AppError> {
        log::info!(
            "[DB] get_usage_model_stats start={:?} end={:?}",
            start_date,
            end_date
        );
        let _start = std::time::Instant::now();
        let conn = self.conn();
        let (where_clause, params_list) = build_ts_where("created_at", start_date, end_date);

        let sql = format!(
            "SELECT model,
                    COUNT(*) as request_count,
                    SUM(input_tokens) as total_input,
                    SUM(output_tokens) as total_output
             FROM proxy_request_logs
             {}
             GROUP BY model
             ORDER BY request_count DESC",
            where_clause
        );

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_list.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query(param_refs.as_slice())?;

        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            results.push(ModelStats {
                model: row.get(0)?,
                request_count: row.get(1)?,
                total_input_tokens: row.get(2)?,
                total_output_tokens: row.get(3)?,
            });
        }
        log::info!(
            "[DB] get_usage_model_stats done: {} models ({}ms)",
            results.len(),
            _start.elapsed().as_millis()
        );
        Ok(results)
    }

    /// Paginated request logs with optional filters.
    pub fn get_request_logs_paginated(
        &self,
        filters: &LogFilters,
        page: u32,
        page_size: u32,
    ) -> Result<PaginatedLogs, AppError> {
        log::info!(
            "[DB] get_request_logs_paginated page={} size={}",
            page,
            page_size
        );
        let _start = std::time::Instant::now();
        let conn = self.conn();

        // Build WHERE clause and params
        let (conditions, mut param_values) = build_log_filters(filters);
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Count total
        let count_sql = format!("SELECT COUNT(*) FROM proxy_request_logs l {}", where_clause);
        let mut count_stmt = conn.prepare(&count_sql)?;
        {
            let refs: Vec<&dyn rusqlite::types::ToSql> =
                param_values.iter().map(|p| p.as_ref()).collect();
            let total: i64 = count_stmt.query_row(refs.as_slice(), |row| row.get(0))?;

            // Query page with optional LEFT JOIN on usage_records for token info
            let offset = (page as i64) * (page_size as i64);
            let limit = page_size as i64;
            let base = param_values.len();

            let data_sql = format!(
                "SELECT l.id, l.app_type, l.provider_id, l.request_path,
                        l.request_model, l.status_code, l.success, l.error_message,
                        l.model, l.input_tokens, l.output_tokens,
                        l.cache_read_tokens, l.cache_creation_tokens,
                        l.total_cost_usd, l.data_source, l.created_at
                 FROM proxy_request_logs l
                 {}
                 ORDER BY l.created_at DESC, l.id DESC
                 LIMIT ?{} OFFSET ?{}",
                where_clause,
                base + 1,
                base + 2
            );

            param_values.push(Box::new(limit));
            param_values.push(Box::new(offset));

            let data_refs: Vec<&dyn rusqlite::types::ToSql> =
                param_values.iter().map(|p| p.as_ref()).collect();
            let mut stmt = conn.prepare(&data_sql)?;
            let mut rows = stmt.query(data_refs.as_slice())?;

            let mut data = Vec::new();
            while let Some(row) = rows.next()? {
                data.push(RequestLogDetail {
                    id: row.get(0)?,
                    app_type: row.get(1)?,
                    provider_id: row.get(2)?,
                    request_path: row.get(3)?,
                    request_model: row.get(4)?,
                    status_code: row.get(5)?,
                    success: row.get::<_, i64>(6)? != 0,
                    error_message: row.get(7)?,
                    model: row.get(8)?,
                    input_tokens: row.get(9)?,
                    output_tokens: row.get(10)?,
                    cache_read_tokens: row.get(11)?,
                    cache_creation_tokens: row.get(12)?,
                    total_cost_usd: row.get(13)?,
                    data_source: row.get(14)?,
                    created_at: row.get(15)?,
                });
            }

            log::info!(
                "[DB] get_request_logs_paginated done: {} rows, total={} ({}ms)",
                data.len(),
                total,
                _start.elapsed().as_millis()
            );
            Ok(PaginatedLogs {
                data,
                total,
                page,
                page_size,
            })
        }
    }

    /// Get detail for a single request log by id.
    pub fn get_request_log_detail(
        &self,
        log_id: i64,
    ) -> Result<Option<RequestLogDetail>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT l.id, l.app_type, l.provider_id, l.request_path,
                    l.request_model, l.status_code, l.success, l.error_message,
                    l.model, l.input_tokens, l.output_tokens,
                    l.cache_read_tokens, l.cache_creation_tokens,
                    l.total_cost_usd, l.data_source, l.created_at
             FROM proxy_request_logs l
             WHERE l.id = ?1",
        )?;

        let mut rows = stmt.query(params![log_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(RequestLogDetail {
                id: row.get(0)?,
                app_type: row.get(1)?,
                provider_id: row.get(2)?,
                request_path: row.get(3)?,
                request_model: row.get(4)?,
                status_code: row.get(5)?,
                success: row.get::<_, i64>(6)? != 0,
                error_message: row.get(7)?,
                model: row.get(8)?,
                input_tokens: row.get(9)?,
                output_tokens: row.get(10)?,
                cache_read_tokens: row.get(11)?,
                cache_creation_tokens: row.get(12)?,
                total_cost_usd: row.get(13)?,
                data_source: row.get(14)?,
                created_at: row.get(15)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Aggregate usage by source app_type from proxy_request_logs.
    pub fn get_usage_by_source(&self) -> Result<Vec<UsageSourceItem>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT app_type, COUNT(*) as request_count
             FROM proxy_request_logs
             GROUP BY app_type
             ORDER BY request_count DESC",
        )?;
        let mut rows = stmt.query([])?;
        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            results.push(UsageSourceItem {
                app_type: row.get(0)?,
                request_count: row.get(1)?,
            });
        }
        Ok(results)
    }

    /// Aggregate usage by source with time filter (created_at >= since).
    pub fn get_usage_by_source_since(&self, since: i64) -> Result<Vec<UsageSourceItem>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT app_type, COUNT(*) as request_count
             FROM proxy_request_logs
             WHERE created_at >= ?1
             GROUP BY app_type
             ORDER BY request_count DESC",
        )?;
        let mut rows = stmt.query(params![since])?;
        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            results.push(UsageSourceItem {
                app_type: row.get(0)?,
                request_count: row.get(1)?,
            });
        }
        Ok(results)
    }

    /// Aggregate usage by source within an optional timestamp range.
    pub fn get_usage_by_source_range(
        &self,
        start_date: Option<i64>,
        end_date: Option<i64>,
    ) -> Result<Vec<UsageSourceItem>, AppError> {
        let conn = self.conn();
        let (where_clause, params_list) = build_ts_where("created_at", start_date, end_date);
        let sql = format!(
            "SELECT app_type, COUNT(*) as request_count
             FROM proxy_request_logs
             {}
             GROUP BY app_type
             ORDER BY request_count DESC",
            where_clause
        );
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_list.iter().map(|param| param.as_ref()).collect();
        let mut rows = stmt.query(param_refs.as_slice())?;
        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            results.push(UsageSourceItem {
                app_type: row.get(0)?,
                request_count: row.get(1)?,
            });
        }
        Ok(results)
    }

    // ── Model pricing CRUD ──

    pub fn get_model_pricing(&self) -> Result<Vec<ModelPricing>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT model_id, display_name, input_cost_per_million, output_cost_per_million,
                    cache_read_cost_per_million, cache_creation_cost_per_million
             FROM model_pricing
             ORDER BY display_name",
        )?;
        let mut rows = stmt.query([])?;
        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            results.push(ModelPricing {
                model_id: row.get(0)?,
                display_name: row.get(1)?,
                input_cost_per_million: row.get(2)?,
                output_cost_per_million: row.get(3)?,
                cache_read_cost_per_million: row.get(4)?,
                cache_creation_cost_per_million: row.get(5)?,
            });
        }
        Ok(results)
    }

    pub fn upsert_model_pricing(&self, pricing: &ModelPricing) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "INSERT OR REPLACE INTO model_pricing
             (model_id, display_name, input_cost_per_million, output_cost_per_million,
              cache_read_cost_per_million, cache_creation_cost_per_million)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                pricing.model_id,
                pricing.display_name,
                pricing.input_cost_per_million,
                pricing.output_cost_per_million,
                pricing.cache_read_cost_per_million,
                pricing.cache_creation_cost_per_million,
            ],
        )?;
        Ok(())
    }

    pub fn delete_model_pricing(&self, model_id: &str) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM model_pricing WHERE model_id = ?1",
            params![model_id],
        )?;
        Ok(())
    }
}

/// Build WHERE clause for timestamp range filtering.
fn build_ts_where(
    column: &str,
    start: Option<i64>,
    end: Option<i64>,
) -> (String, Vec<Box<dyn rusqlite::types::ToSql>>) {
    let mut parts = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(ts) = start {
        parts.push(format!("{column} >= ?{}", params.len() + 1));
        params.push(Box::new(ts));
    }
    if let Some(ts) = end {
        parts.push(format!("{column} <= ?{}", params.len() + 1));
        params.push(Box::new(ts));
    }
    let clause = if parts.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", parts.join(" AND "))
    };
    (clause, params)
}

/// Build WHERE conditions and params from LogFilters.
fn build_log_filters(filters: &LogFilters) -> (Vec<String>, Vec<Box<dyn rusqlite::types::ToSql>>) {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    macro_rules! push {
        ($fmt:expr, $val:expr) => {{
            conditions.push(format!($fmt, params.len() + 1));
            params.push(Box::new($val));
        }};
    }

    if let Some(ref v) = filters.app_type {
        push!("l.app_type = ?{}", v.clone());
    }
    if let Some(ref v) = filters.provider_id {
        push!("l.provider_id = ?{}", v.clone());
    }
    if let Some(ref v) = filters.model {
        push!("l.request_model = ?{}", v.clone());
    }
    if let Some(v) = filters.status_code {
        push!("l.status_code = ?{}", v);
    }
    if let Some(v) = filters.start_date {
        push!("l.created_at >= ?{}", v);
    }
    if let Some(v) = filters.end_date {
        push!("l.created_at <= ?{}", v);
    }

    (conditions, params)
}

#[cfg(test)]
mod tests {
    use super::super::Database;
    use rusqlite::{params, Connection};
    use std::sync::Mutex;

    fn database_with_usage() -> Database {
        let database = Database {
            conn: Mutex::new(Connection::open_in_memory().unwrap()),
        };
        database.create_tables().unwrap();
        let conn = database.conn();
        for (timestamp, provider, source) in [
            (100_i64, "old", "proxy"),
            (200_i64, "included", "session_log"),
            (300_i64, "new", "proxy"),
        ] {
            conn.execute(
                "INSERT INTO proxy_request_logs
                 (app_type, provider_id, request_path, model, input_tokens, output_tokens,
                  success, data_source, created_at)
                 VALUES (?1, ?2, '/v1/messages', 'model', 10, 5, 1, ?3, ?4)",
                params![source, provider, source, timestamp],
            )
            .unwrap();
        }
        drop(conn);
        database
    }

    #[test]
    fn usage_summary_queries_apply_both_range_boundaries() {
        let database = database_with_usage();

        let summary = database
            .get_usage_summary_by_provider(Some(150), Some(250))
            .unwrap();
        let trend = database
            .get_usage_daily_trend(Some(150), Some(250))
            .unwrap();
        let sources = database
            .get_usage_by_source_range(Some(150), Some(250))
            .unwrap();

        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].provider_id, "included");
        assert_eq!(trend.len(), 1);
        assert_eq!(trend[0].request_count, 1);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].app_type, "session_log");
        assert_eq!(sources[0].request_count, 1);
    }
}
