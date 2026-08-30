//! Claude Code session log usage tracker
//!
//! Parses token usage from ~/.claude/projects/*/*.jsonl session files,
//! enabling usage statistics for direct-connect (non-proxy) users.

use crate::config::get_claude_config_dir;
use crate::database::{DataSourceSummary, Database, SessionSyncResult};
use crate::error::AppError;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Parsed usage data from a single assistant message in a JSONL session file
struct ParsedAssistantUsage {
    message_id: String,
    model: String,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_creation_tokens: i64,
    stop_reason: Option<String>,
    timestamp: Option<String>,
}

/// Sync all Claude Code session logs to proxy_request_logs
pub fn sync_claude_session_logs(db: &Database) -> Result<SessionSyncResult, AppError> {
    let claude_dir = get_claude_config_dir();
    let projects_dir = claude_dir.join("projects");

    if !projects_dir.exists() {
        return Ok(SessionSyncResult {
            imported: 0,
            skipped: 0,
            files_scanned: 0,
            errors: vec![],
        });
    }

    let mut result = SessionSyncResult {
        imported: 0,
        skipped: 0,
        files_scanned: 0,
        errors: vec![],
    };

    let jsonl_files = collect_jsonl_files(&projects_dir);

    for file_path in &jsonl_files {
        result.files_scanned += 1;
        match sync_single_file(db, file_path) {
            Ok((imported, skipped)) => {
                result.imported += imported;
                result.skipped += skipped;
            }
            Err(e) => {
                let msg = format!("{}: {}", file_path.display(), e);
                log::warn!("[SessionSync] File parse error: {}", msg);
                result.errors.push(msg);
            }
        }
    }

    if result.imported > 0 {
        log::info!(
            "[SessionSync] Complete: imported={}, skipped={}, files_scanned={}",
            result.imported,
            result.skipped,
            result.files_scanned
        );
    }

    Ok(result)
}

/// Collect all .jsonl files under ~/.claude/projects/<project>/
fn collect_jsonl_files(projects_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let entries = match fs::read_dir(projects_dir) {
        Ok(e) => e,
        Err(_) => return files,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(sub_entries) = fs::read_dir(&path) {
            for sub_entry in sub_entries.flatten() {
                let sub_path = sub_entry.path();
                if sub_path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                    files.push(sub_path);
                }
            }
        }
    }
    files
}

/// Sync a single JSONL file, returns (imported, skipped)
fn sync_single_file(db: &Database, file_path: &Path) -> Result<(u32, u32), AppError> {
    let file_path_str = file_path.to_string_lossy().to_string();

    let metadata = fs::metadata(file_path)
        .map_err(|e| AppError::Config(format!("Cannot read file metadata: {}", e)))?;
    let file_modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // Get sync state (last_modified, last_line_offset)
    let (last_modified, last_offset) = get_sync_state(db, &file_path_str)?;

    // Skip unchanged files
    if file_modified <= last_modified {
        return Ok((0, 0));
    }

    let file = fs::File::open(file_path)
        .map_err(|e| AppError::Config(format!("Cannot open file: {}", e)))?;
    let reader = BufReader::new(file);

    let mut line_offset: i64 = 0;
    let mut messages: HashMap<String, ParsedAssistantUsage> = HashMap::new();

    for line_result in reader.lines() {
        line_offset += 1;
        if line_offset <= last_offset {
            continue;
        }

        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }

        let value: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Only process assistant messages
        if value.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }

        let message = match value.get("message") {
            Some(m) => m,
            None => continue,
        };
        let msg_id = match message.get("id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };
        let usage = match message.get("usage") {
            Some(u) => u,
            None => continue,
        };

        let parsed = ParsedAssistantUsage {
            message_id: msg_id.clone(),
            model: message
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            input_tokens: usage
                .get("input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            output_tokens: usage
                .get("output_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            cache_read_tokens: usage
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            cache_creation_tokens: usage
                .get("cache_creation_input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            stop_reason: message
                .get("stop_reason")
                .and_then(|v| v.as_str())
                .map(String::from),
            timestamp: value
                .get("timestamp")
                .and_then(|v| v.as_str())
                .map(String::from),
        };

        // Dedup by message_id: prefer entry with stop_reason (final state)
        let should_replace = match messages.get(&msg_id) {
            None => true,
            Some(existing) => {
                if parsed.stop_reason.is_some() && existing.stop_reason.is_none() {
                    true
                } else if parsed.stop_reason.is_some() == existing.stop_reason.is_some() {
                    parsed.output_tokens > existing.output_tokens
                } else {
                    false
                }
            }
        };
        if should_replace {
            messages.insert(msg_id, parsed);
        }
    }

    let mut imported = 0u32;
    let mut skipped = 0u32;

    for msg in messages.values() {
        // Only import final messages (those with stop_reason)
        if msg.stop_reason.is_none() {
            continue;
        }
        if msg.output_tokens == 0 {
            continue;
        }

        let request_id = format!("session-{}", msg.message_id);

        match insert_session_log_entry(db, &request_id, msg) {
            Ok(true) => imported += 1,
            Ok(false) => skipped += 1,
            Err(e) => {
                log::warn!("[SessionSync] Insert failed ({}): {}", msg.message_id, e);
                skipped += 1;
            }
        }
    }

    update_sync_state(db, &file_path_str, file_modified, line_offset)?;
    Ok((imported, skipped))
}

/// Check if a session log entry should be skipped (duplicate detection)
fn should_skip_session_insert(
    conn: &rusqlite::Connection,
    request_id: &str,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    created_at: i64,
) -> Result<bool, AppError> {
    // Exact match by request_id
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM proxy_request_logs WHERE request_id = ?1 AND data_source = 'session_log'",
            rusqlite::params![request_id],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    if exists {
        return Ok(true);
    }

    // Check for proxy records within +/-10 second window (cross-source dedup)
    let proxy_dup: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM proxy_request_logs
             WHERE data_source = 'proxy'
               AND model = ?1
               AND input_tokens = ?2
               AND output_tokens = ?3
               AND created_at BETWEEN ?4 AND ?5",
            rusqlite::params![
                model,
                input_tokens,
                output_tokens,
                created_at - 10,
                created_at + 10
            ],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(proxy_dup)
}

/// Extract provider name from model string (first segment before `-`)
fn provider_id_from_model(model: &str) -> String {
    model.split('-').next().unwrap_or(model).to_lowercase()
}

/// Insert one session log entry into proxy_request_logs
fn insert_session_log_entry(
    db: &Database,
    request_id: &str,
    msg: &ParsedAssistantUsage,
) -> Result<bool, AppError> {
    let conn = db.conn();

    let provider_id = provider_id_from_model(&msg.model);

    let created_at = msg
        .timestamp
        .as_ref()
        .and_then(|ts| {
            chrono::DateTime::parse_from_rfc3339(ts)
                .ok()
                .map(|dt| dt.timestamp())
        })
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        });

    if should_skip_session_insert(
        &conn,
        request_id,
        &msg.model,
        msg.input_tokens,
        msg.output_tokens,
        created_at,
    )? {
        return Ok(false);
    }

    // Calculate cost from model_pricing
    let cost = calculate_cost(
        &conn,
        &msg.model,
        msg.input_tokens,
        msg.output_tokens,
        msg.cache_read_tokens,
        msg.cache_creation_tokens,
    );

    conn.execute(
        "INSERT OR IGNORE INTO proxy_request_logs (
            app_type, provider_id, request_path, request_model,
            status_code, success, error_message,
            request_id, model, input_tokens, output_tokens,
            cache_read_tokens, cache_creation_tokens, total_cost_usd, data_source, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        rusqlite::params![
            "claude",
            &provider_id,
            "",
            msg.model,
            200i32,
            1i64,
            Option::<String>::None,
            request_id,
            msg.model,
            msg.input_tokens,
            msg.output_tokens,
            msg.cache_read_tokens,
            msg.cache_creation_tokens,
            cost,
            "session_log",
            created_at,
        ],
    )
    .map_err(|e| AppError::Database(format!("Insert session log failed: {}", e)))?;

    Ok(true)
}

/// Get sync progress for a file
fn get_sync_state(db: &Database, file_path: &str) -> Result<(i64, i64), AppError> {
    let conn = db.conn();
    let result = conn.query_row(
        "SELECT last_modified, last_line_offset FROM session_log_sync WHERE file_path = ?1",
        rusqlite::params![file_path],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    );
    Ok(result.unwrap_or((0, 0)))
}

/// Update sync progress for a file
fn update_sync_state(
    db: &Database,
    file_path: &str,
    last_modified: i64,
    last_offset: i64,
) -> Result<(), AppError> {
    let conn = db.conn();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    conn.execute(
        "INSERT OR REPLACE INTO session_log_sync (file_path, last_modified, last_line_offset, last_synced_at)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![file_path, last_modified, last_offset, now],
    )
    .map_err(|e| AppError::Database(format!("Update sync state failed: {}", e)))?;
    Ok(())
}

/// Get data source breakdown for the usage page
pub fn get_data_source_breakdown(
    db: &Database,
    start_date: Option<i64>,
    end_date: Option<i64>,
) -> Result<Vec<DataSourceSummary>, AppError> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT COALESCE(l.data_source, 'proxy') as ds,
                COUNT(*) as cnt,
                COALESCE(SUM(CAST(l.total_cost_usd AS REAL)), 0) as cost
         FROM proxy_request_logs l
         WHERE (?1 IS NULL OR l.created_at >= ?1)
           AND (?2 IS NULL OR l.created_at <= ?2)
         GROUP BY ds
         ORDER BY cnt DESC",
    )?;
    let rows = stmt.query_map(rusqlite::params![start_date, end_date], |row| {
        Ok(DataSourceSummary {
            data_source: row.get(0)?,
            request_count: row.get::<_, i64>(1)? as u32,
            total_cost_usd: format!("{:.6}", row.get::<_, f64>(2)?),
        })
    })?;
    let mut summaries = Vec::new();
    for row in rows {
        summaries.push(row.map_err(|e| AppError::Database(e.to_string()))?);
    }
    Ok(summaries)
}

// ── Cost calculation ──

/// Calculate cost in USD for a set of tokens given a model
fn calculate_cost(
    conn: &rusqlite::Connection,
    model_id: &str,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_creation_tokens: i64,
) -> String {
    let pricing = find_pricing(conn, model_id);
    let (input_price, output_price, cache_read_price, cache_creation_price) = match pricing {
        Some(p) => p,
        None => return "0".to_string(),
    };

    let input_cost = (input_tokens as f64) / 1_000_000.0 * input_price;
    let output_cost = (output_tokens as f64) / 1_000_000.0 * output_price;
    let cache_read_cost = (cache_read_tokens as f64) / 1_000_000.0 * cache_read_price;
    let cache_creation_cost = (cache_creation_tokens as f64) / 1_000_000.0 * cache_creation_price;

    let total = input_cost + output_cost + cache_read_cost + cache_creation_cost;
    format!("{:.10}", total)
}

fn find_pricing(conn: &rusqlite::Connection, model_id: &str) -> Option<(f64, f64, f64, f64)> {
    if let Some(p) = try_find_pricing(conn, model_id) {
        return Some(p);
    }
    // Strip date suffix: "claude-opus-4-6-20260206" -> "claude-opus-4-6"
    let parts: Vec<&str> = model_id.rsplitn(2, '-').collect();
    if parts.len() == 2 {
        if let Some(suffix) = parts.first() {
            if suffix.len() == 8 && suffix.chars().all(|c| c.is_ascii_digit()) {
                if let Some(p) = try_find_pricing(conn, parts[1]) {
                    return Some(p);
                }
            }
        }
    }
    let pattern = format!("{}%", model_id);
    conn.query_row(
        "SELECT input_cost_per_million, output_cost_per_million,
                cache_read_cost_per_million, cache_creation_cost_per_million
         FROM model_pricing WHERE model_id LIKE ?1 LIMIT 1",
        rusqlite::params![pattern],
        |row| {
            Ok((
                row.get::<_, f64>(0).unwrap_or(0.0),
                row.get::<_, f64>(1).unwrap_or(0.0),
                row.get::<_, f64>(2).unwrap_or(0.0),
                row.get::<_, f64>(3).unwrap_or(0.0),
            ))
        },
    )
    .ok()
}

fn try_find_pricing(conn: &rusqlite::Connection, model_id: &str) -> Option<(f64, f64, f64, f64)> {
    conn.query_row(
        "SELECT input_cost_per_million, output_cost_per_million,
                cache_read_cost_per_million, cache_creation_cost_per_million
         FROM model_pricing WHERE model_id = ?1",
        rusqlite::params![model_id],
        |row| {
            Ok((
                row.get::<_, f64>(0).unwrap_or(0.0),
                row.get::<_, f64>(1).unwrap_or(0.0),
                row.get::<_, f64>(2).unwrap_or(0.0),
                row.get::<_, f64>(3).unwrap_or(0.0),
            ))
        },
    )
    .ok()
}
