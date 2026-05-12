//! Usage handlers

use super::super::state::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use cc_switch_lib::database::{
    DailyUsage, DataSourceSummary, LogFilters, ModelPricing, ModelStats, PaginatedLogs,
    ProviderStats, ProviderUsageSummary, RequestLogDetail, SessionSyncResult, UsageSourceItem,
};
use cc_switch_lib::usage::{sync_claude_session_logs, get_data_source_breakdown};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Instant, SystemTime};

// ── Request types ──

#[derive(Deserialize, Default)]
pub struct DateRangeQuery {
    pub start_date: Option<i64>,
    pub end_date: Option<i64>,
}

#[derive(Deserialize, Default)]
pub struct LogsQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub app_type: Option<String>,
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub status_code: Option<i32>,
    pub start_date: Option<i64>,
    pub end_date: Option<i64>,
}

// ── Response types ──

#[derive(Serialize)]
pub struct UsageTotals {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub request_count: i64,
}

#[derive(Serialize)]
pub struct UsageProviderItem {
    pub provider_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub request_count: i64,
}

#[derive(Serialize)]
pub struct UsageModelItem {
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub request_count: i64,
}

#[derive(Serialize)]
pub struct UsageSummaryResponse {
    pub totals: UsageTotals,
    pub providers: Vec<UsageProviderItem>,
    pub models: Vec<UsageModelItem>,
    pub trend: Vec<DailyUsage>,
    pub sources: Vec<UsageSourceItem>,
}

#[derive(Serialize)]
pub struct ProviderStatsResponse {
    pub providers: Vec<ProviderStats>,
}

#[derive(Serialize)]
pub struct ModelStatsResponse {
    pub models: Vec<ModelStats>,
}

// ── Handlers ──

pub async fn get_usage_summary(State(state): State<Arc<AppState>>) -> Json<UsageSummaryResponse> {
    log::info!("[Usage] get_usage_summary");
    let state = state.clone();
    let start = Instant::now();
    match tokio::task::spawn_blocking(move || {
        let since = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0) - 90 * 86400;
        match state.db.get_usage_summary_by_provider(Some(since)) {
            Ok(summary) => {
                let totals = UsageTotals {
                    input_tokens: summary.iter().map(|s| s.total_input_tokens).sum(),
                    output_tokens: summary.iter().map(|s| s.total_output_tokens).sum(),
                    request_count: summary.iter().map(|s| s.request_count).sum(),
                };
                let providers = aggregate_by_provider(&summary);
                let models = aggregate_by_model(&summary);
                let trend = state.db.get_usage_daily_trend(30).unwrap_or_default();
                let sources = state.db.get_usage_by_source_since(since).unwrap_or_default();
                log::info!(
                    "[Usage] summary done: {} providers, {} models, {} trend days, {} sources ({}ms)",
                    providers.len(),
                    models.len(),
                    trend.len(),
                    sources.len(),
                    start.elapsed().as_millis(),
                );
                Json(UsageSummaryResponse {
                    totals,
                    providers,
                    models,
                    trend,
                    sources,
                })
            }
            Err(e) => {
                log::error!("[Usage] get_usage_summary failed after {}ms: {}", start.elapsed().as_millis(), e);
                Json(UsageSummaryResponse {
                    totals: UsageTotals {
                        input_tokens: 0,
                        output_tokens: 0,
                        request_count: 0,
                    },
                    providers: vec![],
                    models: vec![],
                    trend: vec![],
                    sources: vec![],
                })
            }
        }
    })
    .await
    {
        Ok(response) => response,
        Err(e) => {
            log::error!("[Usage] get_usage_summary join failed after {}ms: {}", start.elapsed().as_millis(), e);
            Json(UsageSummaryResponse {
                totals: UsageTotals {
                    input_tokens: 0,
                    output_tokens: 0,
                    request_count: 0,
                },
                providers: vec![],
                models: vec![],
                trend: vec![],
                sources: vec![],
            })
        }
    }
}

pub async fn get_provider_stats(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DateRangeQuery>,
) -> Json<ProviderStatsResponse> {
    log::info!("[Usage] get_provider_stats start_date={:?} end_date={:?}", query.start_date, query.end_date);
    let state = state.clone();
    let start = Instant::now();
    match tokio::task::spawn_blocking(move || {
        match state
            .db
            .get_usage_provider_stats(query.start_date, query.end_date)
        {
            Ok(providers) => {
                log::info!("[Usage] provider_stats done: {} providers ({}ms)", providers.len(), start.elapsed().as_millis());
                Json(ProviderStatsResponse { providers })
            }
            Err(e) => {
                log::error!("[Usage] get_provider_stats query failed after {}ms: {} (start={:?}, end={:?})", start.elapsed().as_millis(), e, query.start_date, query.end_date);
                Json(ProviderStatsResponse {
                    providers: vec![],
                })
            }
        }
    })
    .await
    {
        Ok(response) => response,
        Err(e) => {
            log::error!("[Usage] get_provider_stats join failed after {}ms: {}", start.elapsed().as_millis(), e);
            Json(ProviderStatsResponse { providers: vec![] })
        }
    }
}

pub async fn get_model_stats(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DateRangeQuery>,
) -> Json<ModelStatsResponse> {
    log::info!("[Usage] get_model_stats start_date={:?} end_date={:?}", query.start_date, query.end_date);
    let state = state.clone();
    let start = Instant::now();
    match tokio::task::spawn_blocking(move || {
        match state
            .db
            .get_usage_model_stats(query.start_date, query.end_date)
        {
            Ok(models) => {
                log::info!("[Usage] model_stats done: {} models ({}ms)", models.len(), start.elapsed().as_millis());
                Json(ModelStatsResponse { models })
            }
            Err(e) => {
                log::error!("[Usage] get_model_stats failed after {}ms: {} (start={:?}, end={:?})", start.elapsed().as_millis(), e, query.start_date, query.end_date);
                Json(ModelStatsResponse {
                    models: vec![],
                })
            }
        }
    })
    .await
    {
        Ok(response) => response,
        Err(e) => {
            log::error!("[Usage] get_model_stats join failed after {}ms: {}", start.elapsed().as_millis(), e);
            Json(ModelStatsResponse { models: vec![] })
        }
    }
}

pub async fn get_request_logs_paginated(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LogsQuery>,
) -> Json<PaginatedLogs> {
    let page = query.page.unwrap_or(0);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

    log::info!(
        "[Usage] get_request_logs_paginated page={} size={} app_type={:?} provider={:?} model={:?} status={:?} start={:?} end={:?}",
        page, page_size, query.app_type, query.provider_id, query.model, query.status_code, query.start_date, query.end_date,
    );

    let filters = LogFilters {
        app_type: query.app_type,
        provider_id: query.provider_id,
        model: query.model,
        status_code: query.status_code,
        start_date: query.start_date,
        end_date: query.end_date,
    };

    let start = Instant::now();
    match state.db.get_request_logs_paginated(&filters, page, page_size) {
        Ok(result) => {
            log::info!("[Usage] request_logs done: {} rows, total={} ({}ms)", result.data.len(), result.total, start.elapsed().as_millis());
            Json(result)
        }
        Err(e) => {
            log::error!("[Usage] get_request_logs_paginated failed after {}ms: {} (page={}, size={})", start.elapsed().as_millis(), e, page, page_size);
            Json(PaginatedLogs {
                data: vec![],
                total: 0,
                page,
                page_size,
            })
        }
    }
}

pub async fn get_request_log_detail(
    State(state): State<Arc<AppState>>,
    Path(log_id): Path<i64>,
) -> Json<Option<RequestLogDetail>> {
    log::info!("[Usage] get_request_log_detail id={}", log_id);
    let start = Instant::now();
    match state.db.get_request_log_detail(log_id) {
        Ok(detail) => {
            log::info!("[Usage] request_log_detail done: found={} ({}ms)", detail.is_some(), start.elapsed().as_millis());
            Json(detail)
        }
        Err(e) => {
            log::error!("[Usage] get_request_log_detail failed after {}ms: {} (id={})", start.elapsed().as_millis(), e, log_id);
            Json(None)
        }
    }
}

// ── Model pricing handlers ──

pub async fn get_model_pricing_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ModelPricing>> {
    log::info!("[Usage] get_model_pricing");
    let start = Instant::now();
    match state.db.get_model_pricing() {
        Ok(list) => {
            log::info!("[Usage] model_pricing done: {} entries ({}ms)", list.len(), start.elapsed().as_millis());
            Json(list)
        }
        Err(e) => {
            log::error!("[Usage] get_model_pricing failed after {}ms: {}", start.elapsed().as_millis(), e);
            Json(vec![])
        }
    }
}

pub async fn upsert_model_pricing_handler(
    State(state): State<Arc<AppState>>,
    Json(pricing): Json<ModelPricing>,
) -> Json<serde_json::Value> {
    log::info!("[Usage] upsert_model_pricing model_id={:?}", pricing.model_id);
    let start = Instant::now();
    match state.db.upsert_model_pricing(&pricing) {
        Ok(_) => {
            log::info!("[Usage] upsert_model_pricing done ({}ms)", start.elapsed().as_millis());
            Json(serde_json::json!({ "success": true }))
        }
        Err(e) => {
            log::error!("[Usage] upsert_model_pricing failed after {}ms: {} (model_id={:?})", start.elapsed().as_millis(), e, pricing.model_id);
            Json(serde_json::json!({ "success": false, "error": e.to_string() }))
        }
    }
}

pub async fn delete_model_pricing_handler(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
) -> Json<serde_json::Value> {
    log::info!("[Usage] delete_model_pricing model_id={}", model_id);
    let start = Instant::now();
    match state.db.delete_model_pricing(&model_id) {
        Ok(_) => {
            log::info!("[Usage] delete_model_pricing done ({}ms)", start.elapsed().as_millis());
            Json(serde_json::json!({ "success": true }))
        }
        Err(e) => {
            log::error!("[Usage] delete_model_pricing failed after {}ms: {} (model_id={})", start.elapsed().as_millis(), e, model_id);
            Json(serde_json::json!({ "success": false, "error": e.to_string() }))
        }
    }
}

// ── Helpers ──

fn aggregate_by_provider(summary: &[ProviderUsageSummary]) -> Vec<UsageProviderItem> {
    let mut map = std::collections::HashMap::<String, UsageProviderItem>::new();
    for item in summary {
        map.entry(item.provider_id.clone())
            .and_modify(|e| {
                e.input_tokens += item.total_input_tokens;
                e.output_tokens += item.total_output_tokens;
                e.request_count += item.request_count;
            })
            .or_insert(UsageProviderItem {
                provider_id: item.provider_id.clone(),
                input_tokens: item.total_input_tokens,
                output_tokens: item.total_output_tokens,
                request_count: item.request_count,
            });
    }
    let mut out: Vec<_> = map.into_values().collect();
    out.sort_by(|a, b| (b.input_tokens + b.output_tokens).cmp(&(a.input_tokens + a.output_tokens)));
    out
}

fn aggregate_by_model(summary: &[ProviderUsageSummary]) -> Vec<UsageModelItem> {
    let mut map = std::collections::HashMap::<String, UsageModelItem>::new();
    for item in summary {
        map.entry(item.model.clone())
            .and_modify(|e| {
                e.input_tokens += item.total_input_tokens;
                e.output_tokens += item.total_output_tokens;
                e.request_count += item.request_count;
            })
            .or_insert(UsageModelItem {
                model: item.model.clone(),
                input_tokens: item.total_input_tokens,
                output_tokens: item.total_output_tokens,
                request_count: item.request_count,
            });
    }
    let mut out: Vec<_> = map.into_values().collect();
    out.sort_by(|a, b| (b.input_tokens + b.output_tokens).cmp(&(a.input_tokens + a.output_tokens)));
    out
}

pub async fn sync_session_usage(
    State(state): State<Arc<AppState>>,
) -> Json<SessionSyncResult> {
    log::info!("[Usage] sync_session_usage");
    let start = Instant::now();
    match sync_claude_session_logs(&state.db) {
        Ok(result) => {
            log::info!("[Usage] sync_session_usage done: imported={} skipped={} scanned={} errors={} ({}ms)",
                result.imported, result.skipped, result.files_scanned, result.errors.len(), start.elapsed().as_millis());
            Json(result)
        }
        Err(e) => {
            log::error!("[Usage] sync_session_usage failed after {}ms: {}", start.elapsed().as_millis(), e);
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
    log::info!("[Usage] get_usage_sources");
    let start = Instant::now();
    match get_data_source_breakdown(&state.db) {
        Ok(sources) => {
            log::info!("[Usage] data_sources done: {} sources ({}ms)", sources.len(), start.elapsed().as_millis());
            Json(sources)
        }
        Err(e) => {
            log::error!("[Usage] get_data_source_breakdown failed after {}ms: {}", start.elapsed().as_millis(), e);
            Json(vec![])
        }
    }
}
