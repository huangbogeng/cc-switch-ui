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
pub struct UsageSummaryResponse {
    pub summary: Vec<ProviderUsageSummary>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_requests: i64,
}

#[derive(Serialize)]
pub struct UsageTrendResponse {
    pub trend: Vec<DailyUsage>,
}

#[derive(Serialize)]
pub struct UsageProvidersResponse {
    pub providers: Vec<ProviderUsageSummary>,
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
            let total_input_tokens: i64 = summary.iter().map(|s| s.total_input_tokens).sum();
            let total_output_tokens: i64 = summary.iter().map(|s| s.total_output_tokens).sum();
            let total_requests: i64 = summary.iter().map(|s| s.request_count).sum();
            Json(UsageSummaryResponse {
                summary,
                total_input_tokens,
                total_output_tokens,
                total_requests,
            })
        }
        Err(e) => {
            log::error!("Failed to get usage summary: {}", e);
            Json(UsageSummaryResponse {
                summary: vec![],
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_requests: 0,
            })
        }
    }
}

pub async fn get_usage_trend(State(state): State<Arc<AppState>>) -> Json<UsageTrendResponse> {
    match state.db.get_usage_daily_trend(30) {
        Ok(trend) => Json(UsageTrendResponse { trend }),
        Err(e) => {
            log::error!("Failed to get usage trend: {}", e);
            Json(UsageTrendResponse { trend: vec![] })
        }
    }
}

pub async fn get_usage_providers(
    State(state): State<Arc<AppState>>,
) -> Json<UsageProvidersResponse> {
    match state.db.get_usage_summary_by_provider(None) {
        Ok(summary) => {
            // Group by provider_id and sum
            let mut by_provider: std::collections::HashMap<String, ProviderUsageSummary> =
                std::collections::HashMap::new();
            for item in summary {
                by_provider
                    .entry(item.provider_id.clone())
                    .and_modify(|e| {
                        e.total_input_tokens += item.total_input_tokens;
                        e.total_output_tokens += item.total_output_tokens;
                        e.request_count += item.request_count;
                    })
                    .or_insert(item);
            }
            let mut providers: Vec<_> = by_provider.into_values().collect();
            providers.sort_by(|a, b| {
                (b.total_input_tokens + b.total_output_tokens)
                    .cmp(&(a.total_input_tokens + a.total_output_tokens))
            });
            Json(UsageProvidersResponse { providers })
        }
        Err(e) => {
            log::error!("Failed to get usage providers: {}", e);
            Json(UsageProvidersResponse { providers: vec![] })
        }
    }
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
