# Usage Monitoring Improvement Design

Date: 2026-05-11

## Background and Motivation

cc-switch-ui's usage monitoring currently has no data for users who connect directly to providers (the primary use case). The only data collection path is proxy forwarding (port 15721), which is secondary usage. The original cc-switch has three data collection paths: proxy recording, session log sync from Claude Code JSONL files, and per-provider JS usage scripts.

This spec addresses the most critical gap: **session log sync from Claude Code's local JSONL session files** (`~/.claude/projects/*/*.jsonl`), plus structural fixes to the database schema that make the existing proxy recording path actually work.

## Scope

- Database schema migration for `proxy_request_logs`
- Session log sync engine (Claude Code only)
- Fix proxy forwarding not writing request logs
- Query layer cleanup (remove fragile LEFT JOIN time-window hack)
- API endpoints for sync trigger and data source breakdown
- Frontend: sync button + DataSourceBar component

## Part 1: Database Schema Changes

### proxy_request_logs — add columns

Current table has 7 business columns. Add token tracking, cost, source attribution, and dedup key:

```sql
ALTER TABLE proxy_request_logs ADD COLUMN request_id TEXT;
ALTER TABLE proxy_request_logs ADD COLUMN model TEXT;
ALTER TABLE proxy_request_logs ADD COLUMN input_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE proxy_request_logs ADD COLUMN output_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE proxy_request_logs ADD COLUMN cache_read_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE proxy_request_logs ADD COLUMN cache_creation_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE proxy_request_logs ADD COLUMN total_cost_usd TEXT NOT NULL DEFAULT '0';
ALTER TABLE proxy_request_logs ADD COLUMN data_source TEXT NOT NULL DEFAULT 'proxy';
CREATE INDEX IF NOT EXISTS idx_proxy_request_logs_request_id ON proxy_request_logs(request_id);
CREATE INDEX IF NOT EXISTS idx_proxy_request_logs_data_source ON proxy_request_logs(data_source);
```

### session_log_sync — new table

Tracks incremental sync progress per file:

```sql
CREATE TABLE IF NOT EXISTS session_log_sync (
    file_path TEXT PRIMARY KEY,
    last_modified INTEGER NOT NULL DEFAULT 0,
    last_line_offset INTEGER NOT NULL DEFAULT 0,
    last_synced_at INTEGER NOT NULL DEFAULT (unixepoch())
);
```

### Design decisions

- `request_id` enables exact dedup between proxy and session sync records
- `total_cost_usd` stored as TEXT to use `rust_decimal` string representation
- `usage_records` table is kept as-is; session sync writes only to `proxy_request_logs`, preserving the existing `get_usage_summary_by_provider` query path
- Data source values: `"proxy"` (via forwarder), `"session_log"` (via session sync)

## Part 2: Session Log Sync Engine

File: `cc-switch-lib/src/usage/session_usage.rs`

### Flow

```
Trigger (manual sync button)
    ↓
Scan ~/.claude/projects/<project>/*.jsonl
    ↓
For each file:
    Read session_log_sync → last_modified + last_line_offset
    ↓
    Skip if file not modified (file_mtime <= last_modified)
    ↓
    Open file, seek to last_line_offset
    ↓
    For each new line (line_offset > last_offset):
        Parse JSONL
        Skip non-assistant messages (type != "assistant")
        Extract message.usage: input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens
        ↓
        Dedup by message.id:
            - If same message.id already exists in memory map:
              - Prefer entry with stop_reason (final state over intermediate)
              - If both have or lack stop_reason, keep larger output_tokens
            - Otherwise, insert
    ↓
    For each deduped entry:
        Skip entries without stop_reason (incomplete API calls)
        Skip entries with output_tokens == 0
        ↓
        Build dedup key: (model, input_tokens, output_tokens, created_at ± 10s)
        Check should_skip_session_insert:
            - EXISTS proxy_request_logs WHERE same request_id? → skip
            - EXISTS proxy_request_logs WHERE data_source='proxy'
              AND same model AND same tokens
              AND created_at within ±10 seconds? → skip
        ↓
        Query model_pricing → calculate cost:
            cost = (input_tokens / 1_000_000.0 * input_price)
                 + (output_tokens / 1_000_000.0 * output_price)
                 + (cache_read_tokens / 1_000_000.0 * cache_read_price)
            Prices read from model_pricing table as f64 strings.
            No cost multiplier for session-synced records (multiplier=1.0).
        ↓
        INSERT INTO proxy_request_logs
        (data_source='session_log', provider_id='_session',
         request_id='session-<message_id>')
        ↓
        imported += 1
    ↓
    Update session_log_sync (last_modified, last_line_offset, last_synced_at)
    ↓
Return SessionSyncResult { imported, skipped, files_scanned, errors }
```

### Model pricing lookup

Reuse existing `model_pricing` table. Lookup strategy (in order):
1. Exact match by `model_id`
2. Strip date suffix (e.g. `claude-opus-4-6-20260206` → `claude-opus-4-6`) and retry
3. LIKE pattern match
4. All failed → cost = "0", record still inserted

### Data flow

```
UsageRecord (from proxy forwarder)
    → writes usage_records (tokens only)
    → triggers proxy_request_logs insert with data_source='proxy'

ParsedAssistantUsage (from session log sync)
    → writes proxy_request_logs with data_source='session_log'
    → does NOT write usage_records

Aggregation queries:
    get_usage_summary_by_provider → reads usage_records (unchanged)
    get_usage_model_stats → reads usage_records (unchanged)
    get_usage_provider_stats → reads proxy_request_logs (changed)
    get_request_logs_paginated → reads proxy_request_logs (changed)
```

### SyncResult types

```rust
#[derive(Serialize)]
pub struct SessionSyncResult {
    pub imported: u32,
    pub skipped: u32,
    pub files_scanned: u32,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
pub struct DataSourceSummary {
    pub data_source: String,
    pub request_count: u32,
    pub total_cost_usd: String,
}
```

## Part 3: Fix Proxy Forwarder Not Writing Request Logs

`cc-switch-server/src/proxy/forwarder.rs` — in the response handling path where `save_usage_record` is called, also call `save_proxy_request_log`:

```rust
// After save_usage_record succeeds:
let log_record = ProxyRequestLogRecord {
    app_type: resolve_app_type(&state).unwrap_or("claude"),
    provider_id: state.provider_id.clone(),
    request_path: path.clone(),
    request_model: Some(record.model.clone()),
    status_code: 200,
    success: true,
    error_message: None,
};
let _ = db.save_proxy_request_log(&log_record);
```

- `app_type` resolved from current active provider's app type (consistent with existing routing logic)
- Error is logged but not propagated (log recording failure should not break the proxy response)
- `request_id` left as None for proxy records (dedup uses timestamp window, not exact ID)

## Part 4: Query Layer Cleanup

Three queries currently JOIN with `usage_records` using a fragile ±5 second time window. Replace with direct reads from `proxy_request_logs`.

### get_request_logs_paginated

```
Before:
    SELECT ... FROM proxy_request_logs l
    LEFT JOIN usage_records u
      ON u.provider_id = l.provider_id
     AND u.request_timestamp BETWEEN l.created_at - 5 AND l.created_at + 5

After:
    SELECT ... FROM proxy_request_logs l
    (input_tokens, output_tokens read directly from l.input_tokens, l.output_tokens)
```

### get_request_log_detail

Same transformation — remove LEFT JOIN, read token fields from `proxy_request_logs`.

### get_usage_provider_stats

```
Before:
    Step 1: COUNT/GROUP BY from proxy_request_logs
    Step 2: SUM tokens from usage_records
    Step 3: Merge by provider_id in HashMap

After:
    Single query from proxy_request_logs:
    SELECT provider_id,
           COUNT(*) as request_count,
           SUM(input_tokens) as total_input_tokens,
           SUM(output_tokens) as total_output_tokens,
           SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
           SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as fail_count
    FROM proxy_request_logs
    [WHERE ...]
    GROUP BY provider_id
```

Keep a fallback: if `proxy_request_logs` has no rows, query `usage_records` for backward compat during migration window.

### get_usage_by_source

Query stays the same (already only reads `app_type` from `proxy_request_logs`).

## Part 5: API Endpoints

Add to `cc-switch-server/src/handlers/usage.rs`:

```
POST /api/usage/sync-session  →  triggers sync, returns SessionSyncResult
GET  /api/usage/sources       →  returns Vec<DataSourceSummary>
```

Routes registered in `cc-switch-server/src/lib.rs`.

## Part 6: Frontend Changes

### New: DataSourceBar component

`cc-switch-ui/src/components/usage/DataSourceBar.tsx`:

- Shows a bar with data source chips: proxy, session_log
- Each chip: icon + name + request count
- Sync button: triggers POST /api/usage/sync-session
- Loading state: button disabled with spinner
- Success: show toast with imported/skipped counts, refetch all usage queries
- Hidden if no data or only proxy source (single source = no breakdown needed)

### Modified: UsagePage.tsx

Add sync button row below the title/date-range row:

```
[🔄 Sync Session Logs]  |  proxy: 1,234  session_log: 567
```

### Modified: useUsage.ts

Add hooks:
- `useSyncSession()` — mutation hook for POST /api/usage/sync-session
- `useDataSourceBreakdown()` — query hook for GET /api/usage/sources

### API client

Add to `cc-switch-ui/src/api/index.ts`:

```typescript
interface SessionSyncResult {
  imported: number;
  skipped: number;
  filesScanned: number;
  errors: string[];
}

interface DataSourceSummary {
  dataSource: string;
  requestCount: number;
  totalCostUsd: string;
}

async function syncSessionUsage(): Promise<SessionSyncResult> { ... }
async function getDataSourceBreakdown(): Promise<DataSourceSummary[]> { ... }
```

## Migration

The `proxy_request_logs` table schema change is backward-compatible (all new columns have defaults). Old rows will show 0 tokens and "proxy" as data_source. No data loss.

Migration strategy:
1. Apply ALTER TABLE statements on app startup (existing pattern: `CREATE TABLE IF NOT EXISTS`)
2. Existing proxy_request_logs rows get default values for new columns
3. Session sync can run immediately after migration

## Non-goals

- JS usage script system (out of scope for this iteration)
- Subscription quota / tiered usage bars on ProviderCard (deferred)
- Daily/monthly provider limit checks (deferred)
- Codex/Gemini session sync (only Claude Code for now)
- i18n
