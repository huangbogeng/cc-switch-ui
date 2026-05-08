//! Usage handlers

use super::super::state::AppState;
use axum::{
    extract::{Query, State},
    Json,
};
use cc_switch_lib::database::{DailyUsage, ProviderUsageSummary, ProxyRequestLogEntry};
use serde::Serialize;
use std::sync::Arc;

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
}

#[derive(Serialize)]
pub struct ProxyRequestLogsResponse {
    pub logs: Vec<ProxyRequestLogEntry>,
}

#[derive(serde::Deserialize)]
pub struct ProxyRequestLogsQuery {
    pub limit: Option<usize>,
}

pub async fn get_usage_summary(State(state): State<Arc<AppState>>) -> Json<UsageSummaryResponse> {
    match state.db.get_usage_summary_by_provider(None) {
        Ok(summary) => {
            let totals = UsageTotals {
                input_tokens: summary.iter().map(|s| s.total_input_tokens).sum(),
                output_tokens: summary.iter().map(|s| s.total_output_tokens).sum(),
                request_count: summary.iter().map(|s| s.request_count).sum(),
            };
            let providers = aggregate_by_provider(&summary);
            let models = aggregate_by_model(&summary);
            let trend = state.db.get_usage_daily_trend(30).unwrap_or_default();
            Json(UsageSummaryResponse {
                totals,
                providers,
                models,
                trend,
            })
        }
        Err(e) => {
            log::error!("Failed to get usage summary: {}", e);
            Json(UsageSummaryResponse {
                totals: UsageTotals {
                    input_tokens: 0,
                    output_tokens: 0,
                    request_count: 0,
                },
                providers: vec![],
                models: vec![],
                trend: vec![],
            })
        }
    }
}

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

pub async fn get_proxy_request_logs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProxyRequestLogsQuery>,
) -> Json<ProxyRequestLogsResponse> {
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);
    match state.db.get_proxy_request_logs(limit) {
        Ok(logs) => Json(ProxyRequestLogsResponse { logs }),
        Err(e) => {
            log::error!("Failed to get proxy request logs: {}", e);
            Json(ProxyRequestLogsResponse { logs: vec![] })
        }
    }
}
