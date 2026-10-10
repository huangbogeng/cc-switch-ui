use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use cc_switch_lib::{managed_items, mcp, skills, DEFAULT_APP_TYPE};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub ids: Vec<String>,
}
fn reply<T: serde::Serialize>(result: Result<T, cc_switch_lib::AppError>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            match error {
                cc_switch_lib::AppError::ProviderInUse(_)
                | cc_switch_lib::AppError::Conflict { .. } => StatusCode::CONFLICT,
                cc_switch_lib::AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Json(serde_json::json!({"error":error.to_string()})),
        )
            .into_response(),
    }
}
pub async fn candidates(State(state): State<Arc<AppState>>, Path(kind): Path<String>) -> Response {
    super::file_mutation(move || {
        reply((|| {
            managed_items::validate_kind(&kind)?;
            if kind == "skill" {
                skills::discover_imports(&state.db, DEFAULT_APP_TYPE)
            } else {
                mcp::discover_imports(&state.db, DEFAULT_APP_TYPE)
            }
        })())
    })
    .await
}
pub async fn removed(State(state): State<Arc<AppState>>, Path(kind): Path<String>) -> Response {
    super::file_mutation(move || reply(managed_items::removed(&state.db, DEFAULT_APP_TYPE, &kind)))
        .await
}
async fn apply(
    state: Arc<AppState>,
    kind: String,
    selection: Selection,
    restore: bool,
) -> Response {
    super::file_mutation(move || {
        let _guard = super::MANAGED_MUTATIONS
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        reply((|| {
            managed_items::validate_kind(&kind)?;
            if selection.ids.len() > 2000 {
                return Err(cc_switch_lib::AppError::InvalidInput(
                    "Select at most 2000 items per operation".into(),
                ));
            }
            cc_switch_lib::managed_deletion::ensure_idle(&state.db)?;
            if restore {
                managed_items::restore(&state.db, DEFAULT_APP_TYPE, &kind, &selection.ids)
            } else if kind == "skill" {
                skills::import_selected(&state.db, DEFAULT_APP_TYPE, &selection.ids)
            } else {
                mcp::import_selected(&state.db, DEFAULT_APP_TYPE, &selection.ids)
            }
        })())
    })
    .await
}
pub async fn import(
    State(state): State<Arc<AppState>>,
    Path(kind): Path<String>,
    Json(selection): Json<Selection>,
) -> Response {
    apply(state, kind, selection, false).await
}
pub async fn restore(
    State(state): State<Arc<AppState>>,
    Path(kind): Path<String>,
    Json(selection): Json<Selection>,
) -> Response {
    apply(state, kind, selection, true).await
}
