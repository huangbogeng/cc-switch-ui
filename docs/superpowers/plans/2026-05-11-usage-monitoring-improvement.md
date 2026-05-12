# Usage Monitoring Improvement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable usage data collection for direct-connect users by syncing Claude Code session logs, and fix the proxy request log recording bug.

**Architecture:** Add columns to `proxy_request_logs` to make it the unified fact table. Write a session sync engine that incrementally parses `~/.claude/projects/*/*.jsonl` and writes to `proxy_request_logs`. Fix the proxy forwarder to also write request logs. Clean up the fragile LEFT JOIN time-window hack in queries.

**Tech Stack:** Rust (rusqlite, serde_json, chrono, tokio), React (TanStack Query, shadcn/ui)

---

### Task 1: Database schema migration — proxy_request_logs + session_log_sync

**Files:**
- Modify: `cc-switch-lib/src/database/mod.rs:107-173` (add session_log_sync CREATE TABLE + new columns in proxy_request_logs)
- Modify: `cc-switch-lib/src/database/migrations.rs` (add migration for new columns)
- Test: manual migration test in mod.rs

- [ ] **Step 1: Add new columns to proxy_request_logs and session_log_sync table in create_tables()**

In `cc-switch-lib/src/database/mod.rs`, after the existing `CREATE TABLE IF NOT EXISTS proxy_request_logs` block (ends around line 128), change the `CREATE TABLE IF NOT EXISTS proxy_request_logs` to the new definition with all columns, then add `CREATE TABLE IF NOT EXISTS session_log_sync`:

```sql
CREATE TABLE IF NOT EXISTS proxy_request_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    app_type TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    request_path TEXT NOT NULL,
    request_model TEXT,
    status_code INTEGER,
    success INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    request_id TEXT,
    model TEXT,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cache_read_tokens INTEGER NOT NULL DEFAULT 0,
    cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
    total_cost_usd TEXT NOT NULL DEFAULT '0',
    data_source TEXT NOT NULL DEFAULT 'proxy',
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
```

Also add indexes:

```sql
CREATE INDEX IF NOT EXISTS idx_proxy_request_logs_request_id ON proxy_request_logs(request_id);
CREATE INDEX IF NOT EXISTS idx_proxy_request_logs_data_source ON proxy_request_logs(data_source);
```

Add `session_log_sync` table (before or after proxy_request_logs):

```sql
CREATE TABLE IF NOT EXISTS session_log_sync (
    file_path TEXT PRIMARY KEY,
    last_modified INTEGER NOT NULL DEFAULT 0,
    last_line_offset INTEGER NOT NULL DEFAULT 0,
    last_synced_at INTEGER NOT NULL DEFAULT (unixepoch())
);
```

- [ ] **Step 2: Add migration for existing databases**

In `cc-switch-lib/src/database/migrations.rs`, extend `migrate_proxy_request_logs_schema` to add the new columns:

```rust
fn migrate_proxy_request_logs_schema(conn: &Connection) -> Result<(), AppError> {
    let columns = table_columns(conn, "proxy_request_logs")?;
    if columns.is_empty() {
        return Ok(());
    }

    let has_column = |name: &str| columns.iter().any(|column| column == name);

    // Existing migrations
    if !has_column("request_path") {
        conn.execute("ALTER TABLE proxy_request_logs ADD COLUMN request_path TEXT NOT NULL DEFAULT ''", [])?;
    }
    if !has_column("request_model") {
        conn.execute("ALTER TABLE proxy_request_logs ADD COLUMN request_model TEXT", [])?;
    }
    if !has_column("status_code") {
        conn.execute("ALTER TABLE proxy_request_logs ADD COLUMN status_code INTEGER", [])?;
    }
    if !has_column("success") {
        conn.execute("ALTER TABLE proxy_request_logs ADD COLUMN success INTEGER NOT NULL DEFAULT 0", [])?;
    }
    if !has_column("error_message") {
        conn.execute("ALTER TABLE proxy_request_logs ADD COLUMN error_message TEXT", [])?;
    }

    // New migrations
    if !has_column("request_id") {
        conn.execute("ALTER TABLE proxy_request_logs ADD COLUMN request_id TEXT", [])?;
    }
    if !has_column("model") {
        conn.execute("ALTER TABLE proxy_request_logs ADD COLUMN model TEXT", [])?;
    }
    if !has_column("input_tokens") {
        conn.execute("ALTER TABLE proxy_request_logs ADD COLUMN input_tokens INTEGER NOT NULL DEFAULT 0", [])?;
    }
    if !has_column("output_tokens") {
        conn.execute("ALTER TABLE proxy_request_logs ADD COLUMN output_tokens INTEGER NOT NULL DEFAULT 0", [])?;
    }
    if !has_column("cache_read_tokens") {
        conn.execute("ALTER TABLE proxy_request_logs ADD COLUMN cache_read_tokens INTEGER NOT NULL DEFAULT 0", [])?;
    }
    if !has_column("cache_creation_tokens") {
        conn.execute("ALTER TABLE proxy_request_logs ADD COLUMN cache_creation_tokens INTEGER NOT NULL DEFAULT 0", [])?;
    }
    if !has_column("total_cost_usd") {
        conn.execute("ALTER TABLE proxy_request_logs ADD COLUMN total_cost_usd TEXT NOT NULL DEFAULT '0'", [])?;
    }
    if !has_column("data_source") {
        conn.execute("ALTER TABLE proxy_request_logs ADD COLUMN data_source TEXT NOT NULL DEFAULT 'proxy'", [])?;
    }

    // Also ensure indexes exist
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_proxy_request_logs_request_id ON proxy_request_logs(request_id);
         CREATE INDEX IF NOT EXISTS idx_proxy_request_logs_data_source ON proxy_request_logs(data_source);"
    )?;

    Ok(())
}
```

Also add a `migrate_session_log_sync` function in the same file:

```rust
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
```

Call these in `run_schema_migrations` or in `create_tables()` after the batch execute. In `cc-switch-lib/src/database/mod.rs`, add the calls after line 178:

```rust
// after line: migrations::run_schema_migrations(&conn)?;
migrations::migrate_session_log_schema(&conn)?;
```

- [ ] **Step 3: Compile check and commit**

Run: `cargo check -p cc-switch-lib`
Expected: OK

```bash
git add cc-switch-lib/src/database/mod.rs cc-switch-lib/src/database/migrations.rs
git commit -m "feat(db): add proxy_request_logs columns and session_log_sync table"
```

---

### Task 2: Update Rust types for new fields

**Files:**
- Modify: `cc-switch-lib/src/database/types.rs:103-241`

- [ ] **Step 1: Update ProxyRequestLogRecord to include new fields**

```rust
#[derive(Debug, Clone)]
pub struct ProxyRequestLogRecord {
    pub app_type: String,
    pub provider_id: String,
    pub request_path: String,
    pub request_model: Option<String>,
    pub status_code: Option<i32>,
    pub success: bool,
    pub error_message: Option<String>,
    pub request_id: Option<String>,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
    pub total_cost_usd: String,
    pub data_source: String,
}
```

- [ ] **Step 2: Update ProxyRequestLogEntry to include display fields**

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ProxyRequestLogEntry {
    pub app_type: String,
    pub provider_id: String,
    pub request_path: String,
    pub request_model: Option<String>,
    pub status_code: Option<i32>,
    pub success: bool,
    pub error_message: Option<String>,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
    pub total_cost_usd: String,
    pub data_source: String,
    pub created_at: i64,
}
```

- [ ] **Step 3: Update RequestLogDetail to read tokens directly (not Optional)**

```rust
#[derive(Debug, Clone, Serialize)]
pub struct RequestLogDetail {
    pub id: i64,
    pub app_type: String,
    pub provider_id: String,
    pub request_path: String,
    pub request_model: Option<String>,
    pub status_code: Option<i32>,
    pub success: bool,
    pub error_message: Option<String>,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
    pub total_cost_usd: String,
    pub data_source: String,
    pub created_at: i64,
}
```

Previously `input_tokens: Option<i64>` and `output_tokens: Option<i64>` because the LEFT JOIN could produce NULL. Now they're direct columns with `NOT NULL DEFAULT 0`.

- [ ] **Step 4: Add SessionSyncResult and DataSourceSummary types**

Add to `cc-switch-lib/src/database/types.rs`:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSyncResult {
    pub imported: u32,
    pub skipped: u32,
    pub files_scanned: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSourceSummary {
    pub data_source: String,
    pub request_count: u32,
    pub total_cost_usd: String,
}
```

- [ ] **Step 5: Export new types from database/mod.rs**

In `cc-switch-lib/src/database/mod.rs`, update the `pub use types::{ ... }` block (line 24-28) to also export `SessionSyncResult` and `DataSourceSummary`.

- [ ] **Step 6: Compile check and commit**

Run: `cargo check -p cc-switch-lib`
Expected: OK

```bash
git add cc-switch-lib/src/database/types.rs cc-switch-lib/src/database/mod.rs
git commit -m "feat(db): update types for proxy_request_logs new fields"
```

---

### Task 3: Fix proxy forwarder — record request logs

**Files:**
- Modify: `cc-switch-server/src/proxy/forwarder.rs`
- Modify: `cc-switch-lib/src/database/usage.rs:30-49` (update INSERT for new columns)

- [ ] **Step 1: Update save_proxy_request_log INSERT for new columns**

In `cc-switch-lib/src/database/usage.rs:30-49`, expand the INSERT statement to include all new columns:

```rust
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
```

- [ ] **Step 2: Fix forwarder to call save_proxy_request_log**

In `cc-switch-server/src/proxy/forwarder.rs`, find the non-streaming usage recording block (around line 527-543). After the `save_usage_record` tokio::spawn block, add the request log recording:

```rust
// Record usage if available
if let Some(mut record) = transform_result.record {
    record.provider_id = state.provider_id.clone();
    let db = state.db.clone();
    let app_type = app_type.to_string();
    let req_path = path.clone();
    let rec_model = record.model.clone();
    tokio::spawn(async move {
        if let Err(e) = db.save_usage_record(&record) {
            log::error!("[Proxy] Failed to save usage record: {}", e);
        } else {
            log::info!(
                "[Proxy] Usage recorded: provider={}, model={}, input={}, output={}",
                record.provider_id,
                record.model,
                record.input_tokens,
                record.output_tokens
            );
        }
        // Also save request log
        let log_record = cc_switch_lib::database::ProxyRequestLogRecord {
            app_type: app_type,
            provider_id: record.provider_id.clone(),
            request_path: req_path,
            request_model: Some(rec_model),
            status_code: Some(200),
            success: true,
            error_message: None,
            request_id: None,
            model: Some(record.model.clone()),
            input_tokens: record.input_tokens,
            output_tokens: record.output_tokens,
            cache_read_tokens: record.cache_read_tokens.unwrap_or(0),
            cache_creation_tokens: 0,
            total_cost_usd: "0".to_string(),
            data_source: "proxy".to_string(),
        };
        if let Err(e) = db.save_proxy_request_log(&log_record) {
            log::error!("[Proxy] Failed to save request log: {}", e);
        }
    });
}
```

For the streaming path (around line 470-498, the `OpenAIChat` format), the usage callback already spawns a task. Extend that tokio::spawn to also save the request log:

```rust
openai_chat_sse_to_anthropic_with_usage(
    upstream_res.bytes_stream(),
    move |usage| {
        let record = cc_switch_lib::database::UsageRecord {
            provider_id: provider_id.clone(),
            model: usage.model,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_read_tokens: None,
            request_timestamp: unix_timestamp(),
        };
        let db = db.clone();
        let log_app_type = app_type.clone();
        let log_provider_id = provider_id.clone();
        let log_path = path.clone();
        tokio::spawn(async move {
            if let Err(e) = db.save_usage_record(&record) {
                log::error!("[Proxy] Failed to save streaming usage record: {}", e);
            } else {
                log::info!(
                    "[Proxy] Streaming usage recorded: provider={}, model={}, input={}, output={}",
                    record.provider_id,
                    record.model,
                    record.input_tokens,
                    record.output_tokens
                );
            }
            // Also save request log for streaming
            let log_record = cc_switch_lib::database::ProxyRequestLogRecord {
                app_type: log_app_type,
                provider_id: log_provider_id,
                request_path: log_path,
                request_model: Some(record.model.clone()),
                status_code: Some(200),
                success: true,
                error_message: None,
                request_id: None,
                model: Some(record.model.clone()),
                input_tokens: record.input_tokens,
                output_tokens: record.output_tokens,
                cache_read_tokens: record.cache_read_tokens.unwrap_or(0),
                cache_creation_tokens: 0,
                total_cost_usd: "0".to_string(),
                data_source: "proxy".to_string(),
            };
            if let Err(e) = db.save_proxy_request_log(&log_record) {
                log::error!("[Proxy] Failed to save streaming request log: {}", e);
            }
        });
    },
)
```

Note: `app_type` is available in `forward_with_retry` as a parameter. You need to pass it through to `forward_once` or get it from the state. Check if `state` has an `app_type` field. If not, the simplest approach is to pass `app_type: String` as a parameter to `forward_once`.

Check the `ProxyState` struct definition to see if it has `app_type`:

```bash
grep -n "struct ProxyState" /home/huangbogeng/github.com/newwork/cc-switch-ui/cc-switch-server/src/proxy/mod.rs
```

If not, add a `app_type` field to `ProxyState` or pass it as a parameter. Let me check the ProxyState struct:

Actually, looking at the existing code, `forward_once` is called from `forward_with_retry` which already has `app_type: &str`. So the cleanest path is to capture `app_type` as a String before calling `forward_once` and move it into the closure.

- [ ] **Step 3: Compile check and commit**

Run: `cargo check`
Expected: OK

```bash
git add cc-switch-server/src/proxy/forwarder.rs cc-switch-lib/src/database/usage.rs
git commit -m "fix(proxy): record request logs in proxy forwarder"
```

---

### Task 4: Session log sync engine

**Files:**
- Create: `cc-switch-lib/src/usage/session_usage.rs`
- Modify: `cc-switch-lib/src/usage/mod.rs`
- Modify: `cc-switch-lib/src/lib.rs`

- [ ] **Step 1: Create session_usage.rs with Claude JSONL parser + sync logic**

Create `cc-switch-lib/src/usage/session_usage.rs`:

```rust
//! Claude Code session log usage tracker
//!
//! Parses token usage from ~/.claude/projects/*/*.jsonl session files,
//! enabling usage statistics for direct-connect (non-proxy) users.

use crate::database::{Database, ProxyRequestLogRecord, SessionSyncResult, DataSourceSummary};
use crate::error::AppError;
use crate::config::get_claude_config_dir;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::cost_calculator::calculate_cost;

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
    session_id: Option<String>,
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
        if !path.is_dir() { continue; }
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
    let mut current_session_id: Option<String> = None;

    for line_result in reader.lines() {
        line_offset += 1;
        if line_offset <= last_offset { continue; }

        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() { continue; }

        let value: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Extract session ID
        if current_session_id.is_none() {
            if let Some(sid) = value.get("sessionId").and_then(|v| v.as_str()) {
                current_session_id = Some(sid.to_string());
            }
        }

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
            model: message.get("model").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
            input_tokens: usage.get("input_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
            output_tokens: usage.get("output_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
            cache_read_tokens: usage.get("cache_read_input_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
            cache_creation_tokens: usage.get("cache_creation_input_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
            stop_reason: message.get("stop_reason").and_then(|v| v.as_str()).map(String::from),
            timestamp: value.get("timestamp").and_then(|v| v.as_str()).map(String::from),
            session_id: current_session_id.clone(),
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
        if msg.stop_reason.is_none() { continue; }
        if msg.output_tokens == 0 { continue; }

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
    cache_read_tokens: i64,
    cache_creation_tokens: i64,
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
    if exists { return Ok(true); }

    // Check for proxy records within ±10 second window (cross-source dedup)
    let proxy_dup: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM proxy_request_logs
             WHERE data_source = 'proxy'
               AND model = ?1
               AND input_tokens = ?2
               AND output_tokens = ?3
               AND created_at BETWEEN ?4 AND ?5",
            rusqlite::params![model, input_tokens, output_tokens, created_at - 10, created_at + 10],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(proxy_dup)
}

/// Insert one session log entry into proxy_request_logs
fn insert_session_log_entry(
    db: &Database,
    request_id: &str,
    msg: &ParsedAssistantUsage,
) -> Result<bool, AppError> {
    let conn = db.conn();

    let created_at = msg
        .timestamp
        .as_ref()
        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok().map(|dt| dt.timestamp()))
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
        msg.cache_read_tokens,
        msg.cache_creation_tokens,
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
            "_session",
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
fn update_sync_state(db: &Database, file_path: &str, last_modified: i64, last_offset: i64) -> Result<(), AppError> {
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
pub fn get_data_source_breakdown(db: &Database) -> Result<Vec<DataSourceSummary>, AppError> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT COALESCE(l.data_source, 'proxy') as ds,
                COUNT(*) as cnt,
                COALESCE(SUM(CAST(l.total_cost_usd AS REAL)), 0) as cost
         FROM proxy_request_logs l
         GROUP BY ds
         ORDER BY cnt DESC"
    )?;
    let rows = stmt.query_map([], |row| {
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
```

- [ ] **Step 2: Create cost_calculator.rs**

Create `cc-switch-lib/src/usage/cost_calculator.rs`:

```rust
//! Cost calculation using model_pricing table

use rusqlite::Connection;

/// Calculate cost in USD for a set of tokens given a model
/// Returns the cost as a decimal string
pub fn calculate_cost(
    conn: &Connection,
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

fn find_pricing(conn: &Connection, model_id: &str) -> Option<(f64, f64, f64, f64)> {
    // Exact match
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

    // LIKE match
    let pattern = format!("{}%", model_id);
    let result = conn.query_row(
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
    );
    result.ok()
}

fn try_find_pricing(conn: &Connection, model_id: &str) -> Option<(f64, f64, f64, f64)> {
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
    ).ok()
}
```

- [ ] **Step 3: Register module in usage/mod.rs**

In `cc-switch-lib/src/usage/mod.rs`, add the new modules:

```rust
//! Usage tracking module
//!
//! Parses usage information from API responses and records to database.

mod parser;
mod session_usage;
mod cost_calculator;

pub use parser::UsageParser;
pub use session_usage::{sync_claude_session_logs, get_data_source_breakdown};
pub use cost_calculator::calculate_cost;

use crate::database::UsageRecord;

/// Trait for extracting usage from responses
pub trait UsageExtractor: Send + Sync {
    /// Parse usage from a response body
    fn extract(&self, body: &[u8]) -> Option<UsageRecord>;
}
```

- [ ] **Step 4: Add get_claude_config_dir to config.rs**

In `cc-switch-lib/src/config.rs`, check if there's already a function for the Claude config dir. If not, add one:

```rust
/// Get the Claude Code config directory (~/.claude)
pub fn get_claude_config_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home).join(".claude")
}
```

- [ ] **Step 5: Compile check**

Run: `cargo check -p cc-switch-lib`
Expected: OK

- [ ] **Step 6: Commit**

```bash
git add cc-switch-lib/src/usage/session_usage.rs cc-switch-lib/src/usage/cost_calculator.rs cc-switch-lib/src/usage/mod.rs cc-switch-lib/src/config.rs
git commit -m "feat(usage): add Claude Code session log sync engine"
```

---

### Task 5: Query layer cleanup — remove LEFT JOIN time-window hack

**Files:**
- Modify: `cc-switch-lib/src/database/usage.rs:257-370`

- [ ] **Step 1: Rewrite get_request_logs_paginated to read from proxy_request_logs directly**

Replace the current function body (lines 257-335). The SELECT no longer needs LEFT JOIN:

```rust
pub fn get_request_logs_paginated(
    &self,
    filters: &LogFilters,
    page: u32,
    page_size: u32,
) -> Result<PaginatedLogs, AppError> {
    let conn = self.conn();

    let (conditions, mut param_values) = build_log_filters(filters);
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count total
    let count_sql = format!("SELECT COUNT(*) FROM proxy_request_logs l {}", where_clause);
    let mut count_stmt = conn.prepare(&count_sql)?;
    let refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    let total: i64 = count_stmt.query_row(refs.as_slice(), |row| row.get(0))?;

    // Query page
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

    Ok(PaginatedLogs { data, total, page, page_size })
}
```

- [ ] **Step 2: Rewrite get_request_log_detail**

```rust
pub fn get_request_log_detail(&self, log_id: i64) -> Result<Option<RequestLogDetail>, AppError> {
    let conn = self.conn();
    let mut stmt = conn.prepare(
        "SELECT l.id, l.app_type, l.provider_id, l.request_path,
                l.request_model, l.status_code, l.success, l.error_message,
                l.model, l.input_tokens, l.output_tokens,
                l.cache_read_tokens, l.cache_creation_tokens,
                l.total_cost_usd, l.data_source, l.created_at
         FROM proxy_request_logs l
         WHERE l.id = ?1"
    )?;

    let mut rows = stmt.query(rusqlite::params![log_id])?;
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
```

- [ ] **Step 3: Rewrite get_usage_provider_stats to single query**

```rust
/// Aggregate provider stats from proxy_request_logs.
pub fn get_usage_provider_stats(
    &self,
    start_date: Option<i64>,
    end_date: Option<i64>,
) -> Result<Vec<ProviderStats>, AppError> {
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

    // Fallback to usage_records if proxy_request_logs is empty
    if results.is_empty() {
        return self.get_usage_provider_stats_fallback(start_date, end_date);
    }

    Ok(results)
}

/// Fallback: aggregate from usage_records (for backward compat during migration)
fn get_usage_provider_stats_fallback(
    &self,
    start_date: Option<i64>,
    end_date: Option<i64>,
) -> Result<Vec<ProviderStats>, AppError> {
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
    Ok(results)
}
```

- [ ] **Step 4: Update get_proxy_request_logs SELECT for new columns**

Update the existing `get_proxy_request_logs` function (line 51-74) to also return the new display fields (model, input_tokens, etc.):

```rust
pub fn get_proxy_request_logs(&self, limit: usize) -> Result<Vec<ProxyRequestLogEntry>, AppError> {
    let conn = self.conn();
    let mut stmt = conn.prepare(
        "SELECT app_type, provider_id, request_path, request_model,
                status_code, success, error_message,
                model, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens,
                total_cost_usd, data_source, created_at
         FROM proxy_request_logs
         ORDER BY created_at DESC, id DESC
         LIMIT ?1"
    )?;
    let mut rows = stmt.query(rusqlite::params![limit as i64])?;
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
            model: row.get(7)?,
            input_tokens: row.get(8)?,
            output_tokens: row.get(9)?,
            cache_read_tokens: row.get(10)?,
            cache_creation_tokens: row.get(11)?,
            total_cost_usd: row.get(12)?,
            data_source: row.get(13)?,
            created_at: row.get(14)?,
        });
    }
    Ok(entries)
}
```

- [ ] **Step 5: Compile check and commit**

Run: `cargo check -p cc-switch-lib`
Expected: OK

```bash
git add cc-switch-lib/src/database/usage.rs
git commit -m "refactor(usage): remove LEFT JOIN time-window, read tokens directly"
```

---

### Task 6: API endpoints — sync-session and sources

**Files:**
- Modify: `cc-switch-server/src/handlers/usage.rs`
- Modify: `cc-switch-server/src/lib.rs:377-397`

- [ ] **Step 1: Add sync-session and sources handlers**

In `cc-switch-server/src/handlers/usage.rs`, add after the existing handlers:

```rust
use cc_switch_lib::usage::{sync_claude_session_logs, get_data_source_breakdown};
use cc_switch_lib::database::{SessionSyncResult, DataSourceSummary};

pub async fn sync_session_usage(
    State(state): State<Arc<AppState>>,
) -> Json<SessionSyncResult> {
    match sync_claude_session_logs(&state.db) {
        Ok(result) => Json(result),
        Err(e) => {
            log::error!("Failed to sync session usage: {}", e);
            Json(SessionSyncResult {
                imported: 0,
                skipped: 0,
                files_scanned: 0,
                errors: vec![e.to_string()],
            })
        }
    }
}

pub async fn get_usage_sources(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<DataSourceSummary>> {
    match get_data_source_breakdown(&state.db) {
        Ok(sources) => Json(sources),
        Err(e) => {
            log::error!("Failed to get data sources: {}", e);
            Json(vec![])
        }
    }
}
```

- [ ] **Step 2: Register routes**

In `cc-switch-server/src/lib.rs`, add two routes after the existing usage routes (around line 396):

```rust
.route("/api/usage/sync-session", post(usage::sync_session_usage))
.route("/api/usage/sources", get(usage::get_usage_sources))
```

- [ ] **Step 3: Compile check and commit**

Run: `cargo check`
Expected: OK

```bash
git add cc-switch-server/src/handlers/usage.rs cc-switch-server/src/lib.rs
git commit -m "feat(api): add sync-session and data sources endpoints"
```

---

### Task 7: Frontend API client + hooks

**Files:**
- Modify: `cc-switch-ui/src/api/index.ts`
- Modify: `cc-switch-ui/src/lib/useUsage.ts`

- [ ] **Step 1: Add API functions in api/index.ts**

Add after line 444 (after `getRequestLogDetail`):

```typescript
// Session usage sync
export interface SessionSyncResult {
  imported: number;
  skipped: number;
  filesScanned: number;
  errors: string[];
}

export interface DataSourceSummary {
  dataSource: string;
  requestCount: number;
  totalCostUsd: string;
}

export async function syncSessionUsage() {
  return api<SessionSyncResult>('/usage/sync-session', {
    method: 'POST',
  });
}

export async function getDataSourceBreakdown() {
  return api<DataSourceSummary[]>('/usage/sources');
}
```

- [ ] **Step 2: Add hooks in useUsage.ts**

Add after line 100 (before the closing of the file, after `useDeleteModelPricing`):

```typescript
import {
  syncSessionUsage,
  getDataSourceBreakdown,
  type SessionSyncResult,
  type DataSourceSummary,
} from '@/api';

export function useSyncSession() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: syncSessionUsage,
    onSuccess: () => {
      // Refetch all usage data after sync
      qc.invalidateQueries({ queryKey: usageKeys.all });
    },
  });
}

export function useDataSourceBreakdown() {
  return useQuery({
    queryKey: [...usageKeys.all, 'sources'] as const,
    queryFn: getDataSourceBreakdown,
    refetchInterval: 30_000,
  });
}
```

- [ ] **Step 3: Type check and commit**

Run: `npx tsc --noEmit` (from cc-switch-ui dir)
Expected: OK

```bash
git add cc-switch-ui/src/api/index.ts cc-switch-ui/src/lib/useUsage.ts
git commit -m "feat(ui): add sync-session and data sources API + hooks"
```

---

### Task 8: Frontend DataSourceBar component

**Files:**
- Create: `cc-switch-ui/src/components/usage/DataSourceBar.tsx`

- [ ] **Step 1: Create DataSourceBar component**

```tsx
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { RefreshCw, Database, FileText } from 'lucide-react';
import { useDataSourceBreakdown, useSyncSession, type DataSourceSummary } from '@/lib/useUsage';

const SOURCE_ICONS: Record<string, React.ReactNode> = {
  proxy: <Database className="h-3 w-3" />,
  session_log: <FileText className="h-3 w-3" />,
};

const SOURCE_LABELS: Record<string, string> = {
  proxy: 'Proxy',
  session_log: 'Session Log',
};

export default function DataSourceBar() {
  const { data: sources, isLoading } = useDataSourceBreakdown();
  const syncMutation = useSyncSession();

  if (isLoading || !sources || sources.length <= 1) {
    return null;
  }

  const handleSync = () => {
    syncMutation.mutate();
  };

  return (
    <div className="flex items-center justify-between rounded-lg border bg-muted/30 px-4 py-2 text-sm text-muted-foreground">
      <div className="flex items-center gap-3">
        <span className="text-xs font-medium">Data Sources:</span>
        {sources.map((source) => (
          <Badge
            key={source.dataSource}
            variant="secondary"
            className="gap-1 px-2 py-0.5 text-xs font-normal"
          >
            {SOURCE_ICONS[source.dataSource] || <Database className="h-3 w-3" />}
            {SOURCE_LABELS[source.dataSource] || source.dataSource}
            <span className="font-mono tabular-nums">{source.requestCount.toLocaleString()}</span>
          </Badge>
        ))}
      </div>
      <Button
        variant="outline"
        size="sm"
        className="h-7 gap-1.5 text-xs"
        onClick={handleSync}
        disabled={syncMutation.isPending}
      >
        <RefreshCw
          className={`h-3 w-3 ${syncMutation.isPending ? 'animate-spin' : ''}`}
        />
        {syncMutation.isPending ? 'Syncing...' : 'Sync Session Logs'}
      </Button>
    </div>
  );
}
```

- [ ] **Step 2: Type check and commit**

Run: `npx tsc --noEmit` (from cc-switch-ui dir)
Expected: OK

```bash
git add cc-switch-ui/src/components/usage/DataSourceBar.tsx
git commit -m "feat(ui): add DataSourceBar component with sync button"
```

---

### Task 9: Frontend UsagePage integration

**Files:**
- Modify: `cc-switch-ui/src/pages/UsagePage.tsx`

- [ ] **Step 1: Add DataSourceBar to UsagePage**

Import and render DataSourceBar between the summary cards + trend chart and the tabs:

```tsx
import DataSourceBar from '@/components/usage/DataSourceBar';

// Inside the component, after UsageTrendChart:
<DataSourceBar />

// Full layout:
// <PageHeader />
// <UsageSummaryCards />
// <UsageTrendChart />
// <DataSourceBar />          ← NEW
// <Tabs> ... </Tabs>
```

- [ ] **Step 2: Type check and commit**

Run: `npx tsc --noEmit`
Expected: OK

```bash
git add cc-switch-ui/src/pages/UsagePage.tsx
git commit -m "feat(ui): integrate DataSourceBar into UsagePage"
```

---

### Self-review checklist

- [ ] **Spec coverage**: All spec sections covered:
  - Part 1 (DB schema) → Task 1
  - Part 2 (session sync engine) → Task 4
  - Part 3 (fix proxy forwarder) → Task 3
  - Part 4 (query cleanup) → Task 5
  - Part 5 (API endpoints) → Task 6
  - Part 6 (frontend) → Tasks 7-9

- [ ] **Placeholder scan**: No TBD, TODO, or incomplete steps

- [ ] **Type consistency**: 
  - `RequestLogDetail` field `input_tokens` changed from `Option<i64>` to `i64` → check frontend uses `number` not `number | null`
  - `SessionSyncResult` and `DataSourceSummary` types match between Rust and TypeScript
  - All SELECT column orders match struct field orders in Task 5

- [ ] **Dependency order**: Tasks 1-2 (types) → 3-5 (backend logic) → 6 (API) → 7-9 (frontend). Correct.
