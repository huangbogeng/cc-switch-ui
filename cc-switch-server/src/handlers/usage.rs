//! Usage handlers

use super::super::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use cc_switch_lib::database::{
    DailyUsage, LogFilters, ModelPricing, ModelStats, ProviderStats, ProviderUsageSummary,
    UsageSourceItem,
};
use cc_switch_lib::usage::{get_data_source_breakdown, sync_claude_session_logs};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

fn internal_error(error: impl std::fmt::Display) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": error.to_string() })),
    )
        .into_response()
}

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

pub async fn get_usage_summary(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DateRangeQuery>,
) -> impl IntoResponse {
    log::info!(
        "[Usage] get_usage_summary start_date={:?} end_date={:?}",
        query.start_date,
        query.end_date
    );
    let state = state.clone();
    let start = Instant::now();
    let result = tokio::task::spawn_blocking(move || -> Result<UsageSummaryResponse, String> {
        let summary = state
            .db
            .get_usage_summary_by_provider(query.start_date, query.end_date)
            .map_err(|error| error.to_string())?;
        let totals = UsageTotals {
            input_tokens: summary.iter().map(|item| item.total_input_tokens).sum(),
            output_tokens: summary.iter().map(|item| item.total_output_tokens).sum(),
            request_count: summary.iter().map(|item| item.request_count).sum(),
        };
        let providers = aggregate_by_provider(&summary);
        let models = aggregate_by_model(&summary);
        let trend = state
            .db
            .get_usage_daily_trend(query.start_date, query.end_date)
            .map_err(|error| error.to_string())?;
        let sources = state
            .db
            .get_usage_by_source_range(query.start_date, query.end_date)
            .map_err(|error| error.to_string())?;
        Ok(UsageSummaryResponse {
            totals,
            providers,
            models,
            trend,
            sources,
        })
    })
    .await;
    match result {
        Ok(Ok(response)) => {
            log::info!(
                "[Usage] summary done: {} providers, {} models, {} trend days, {} sources ({}ms)",
                response.providers.len(),
                response.models.len(),
                response.trend.len(),
                response.sources.len(),
                start.elapsed().as_millis(),
            );
            Json(response).into_response()
        }
        Ok(Err(error)) => {
            log::error!(
                "[Usage] get_usage_summary failed after {}ms: {}",
                start.elapsed().as_millis(),
                error
            );
            internal_error(error)
        }
        Err(e) => {
            log::error!(
                "[Usage] get_usage_summary join failed after {}ms: {}",
                start.elapsed().as_millis(),
                e
            );
            internal_error(e)
        }
    }
}

pub async fn get_provider_stats(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DateRangeQuery>,
) -> impl IntoResponse {
    log::info!(
        "[Usage] get_provider_stats start_date={:?} end_date={:?}",
        query.start_date,
        query.end_date
    );
    let state = state.clone();
    let start = Instant::now();
    match tokio::task::spawn_blocking(move || {
        match state
            .db
            .get_usage_provider_stats(query.start_date, query.end_date)
        {
            Ok(providers) => {
                log::info!(
                    "[Usage] provider_stats done: {} providers ({}ms)",
                    providers.len(),
                    start.elapsed().as_millis()
                );
                Json(ProviderStatsResponse { providers }).into_response()
            }
            Err(e) => {
                log::error!(
                    "[Usage] get_provider_stats query failed after {}ms: {} (start={:?}, end={:?})",
                    start.elapsed().as_millis(),
                    e,
                    query.start_date,
                    query.end_date
                );
                internal_error(e)
            }
        }
    })
    .await
    {
        Ok(response) => response,
        Err(e) => {
            log::error!(
                "[Usage] get_provider_stats join failed after {}ms: {}",
                start.elapsed().as_millis(),
                e
            );
            internal_error(e)
        }
    }
}

pub async fn get_model_stats(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DateRangeQuery>,
) -> impl IntoResponse {
    log::info!(
        "[Usage] get_model_stats start_date={:?} end_date={:?}",
        query.start_date,
        query.end_date
    );
    let state = state.clone();
    let start = Instant::now();
    match tokio::task::spawn_blocking(move || {
        match state
            .db
            .get_usage_model_stats(query.start_date, query.end_date)
        {
            Ok(models) => {
                log::info!(
                    "[Usage] model_stats done: {} models ({}ms)",
                    models.len(),
                    start.elapsed().as_millis()
                );
                Json(ModelStatsResponse { models }).into_response()
            }
            Err(e) => {
                log::error!(
                    "[Usage] get_model_stats failed after {}ms: {} (start={:?}, end={:?})",
                    start.elapsed().as_millis(),
                    e,
                    query.start_date,
                    query.end_date
                );
                internal_error(e)
            }
        }
    })
    .await
    {
        Ok(response) => response,
        Err(e) => {
            log::error!(
                "[Usage] get_model_stats join failed after {}ms: {}",
                start.elapsed().as_millis(),
                e
            );
            internal_error(e)
        }
    }
}

pub async fn get_request_logs_paginated(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LogsQuery>,
) -> impl IntoResponse {
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
    match state
        .db
        .get_request_logs_paginated(&filters, page, page_size)
    {
        Ok(result) => {
            log::info!(
                "[Usage] request_logs done: {} rows, total={} ({}ms)",
                result.data.len(),
                result.total,
                start.elapsed().as_millis()
            );
            Json(result).into_response()
        }
        Err(e) => {
            log::error!(
                "[Usage] get_request_logs_paginated failed after {}ms: {} (page={}, size={})",
                start.elapsed().as_millis(),
                e,
                page,
                page_size
            );
            internal_error(e)
        }
    }
}

pub async fn get_request_log_detail(
    State(state): State<Arc<AppState>>,
    Path(log_id): Path<i64>,
) -> impl IntoResponse {
    log::info!("[Usage] get_request_log_detail id={}", log_id);
    let start = Instant::now();
    match state.db.get_request_log_detail(log_id) {
        Ok(detail) => {
            log::info!(
                "[Usage] request_log_detail done: found={} ({}ms)",
                detail.is_some(),
                start.elapsed().as_millis()
            );
            Json(detail).into_response()
        }
        Err(e) => {
            log::error!(
                "[Usage] get_request_log_detail failed after {}ms: {} (id={})",
                start.elapsed().as_millis(),
                e,
                log_id
            );
            internal_error(e)
        }
    }
}

// ── Model pricing handlers ──

pub async fn get_model_pricing_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[Usage] get_model_pricing");
    let start = Instant::now();
    match state.db.get_model_pricing() {
        Ok(list) => {
            log::info!(
                "[Usage] model_pricing done: {} entries ({}ms)",
                list.len(),
                start.elapsed().as_millis()
            );
            Json(list).into_response()
        }
        Err(e) => {
            log::error!(
                "[Usage] get_model_pricing failed after {}ms: {}",
                start.elapsed().as_millis(),
                e
            );
            internal_error(e)
        }
    }
}

pub async fn upsert_model_pricing_handler(
    State(state): State<Arc<AppState>>,
    Json(pricing): Json<ModelPricing>,
) -> impl IntoResponse {
    log::info!(
        "[Usage] upsert_model_pricing model_id={:?}",
        pricing.model_id
    );
    let start = Instant::now();
    match state.db.upsert_model_pricing(&pricing) {
        Ok(_) => {
            log::info!(
                "[Usage] upsert_model_pricing done ({}ms)",
                start.elapsed().as_millis()
            );
            Json(serde_json::json!({ "success": true })).into_response()
        }
        Err(e) => {
            log::error!(
                "[Usage] upsert_model_pricing failed after {}ms: {} (model_id={:?})",
                start.elapsed().as_millis(),
                e,
                pricing.model_id
            );
            internal_error(e)
        }
    }
}

pub async fn delete_model_pricing_handler(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    log::info!("[Usage] delete_model_pricing model_id={}", model_id);
    let start = Instant::now();
    match state.db.delete_model_pricing(&model_id) {
        Ok(_) => {
            log::info!(
                "[Usage] delete_model_pricing done ({}ms)",
                start.elapsed().as_millis()
            );
            Json(serde_json::json!({ "success": true })).into_response()
        }
        Err(e) => {
            log::error!(
                "[Usage] delete_model_pricing failed after {}ms: {} (model_id={})",
                start.elapsed().as_millis(),
                e,
                model_id
            );
            internal_error(e)
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
    out.sort_by_key(|item| std::cmp::Reverse(item.input_tokens + item.output_tokens));
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
    out.sort_by_key(|item| std::cmp::Reverse(item.input_tokens + item.output_tokens));
    out
}

pub async fn sync_session_usage(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[Usage] sync_session_usage");
    let start = Instant::now();
    match sync_claude_session_logs(&state.db) {
        Ok(result) => {
            log::info!("[Usage] sync_session_usage done: imported={} skipped={} scanned={} errors={} ({}ms)",
                result.imported, result.skipped, result.files_scanned, result.errors.len(), start.elapsed().as_millis());
            Json(result).into_response()
        }
        Err(e) => {
            log::error!(
                "[Usage] sync_session_usage failed after {}ms: {}",
                start.elapsed().as_millis(),
                e
            );
            internal_error(e)
        }
    }
}

pub async fn get_usage_sources(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DateRangeQuery>,
) -> impl IntoResponse {
    log::info!(
        "[Usage] get_usage_sources start_date={:?} end_date={:?}",
        query.start_date,
        query.end_date
    );
    let start = Instant::now();
    match get_data_source_breakdown(&state.db, query.start_date, query.end_date) {
        Ok(sources) => {
            log::info!(
                "[Usage] data_sources done: {} sources ({}ms)",
                sources.len(),
                start.elapsed().as_millis()
            );
            Json(sources).into_response()
        }
        Err(e) => {
            log::error!(
                "[Usage] get_data_source_breakdown failed after {}ms: {}",
                start.elapsed().as_millis(),
                e
            );
            internal_error(e)
        }
    }
}
