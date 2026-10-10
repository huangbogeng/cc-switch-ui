pub mod auth;
pub mod backups;
pub mod copilot_oauth;
pub mod managed_deletion;
pub mod mcp;

static MANAGED_MUTATIONS: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub mod model_fetch;
pub mod oauth;
pub mod providers;
pub mod settings;
pub mod skills;
pub mod usage;

/// Filesystem operations and their serialization must not block Axum workers.
async fn file_mutation(
    task: impl FnOnce() -> axum::response::Response + Send + 'static,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    match tokio::task::spawn_blocking(task).await {
        Ok(response) => response,
        Err(error) => {
            log::error!("File mutation task failed: {error}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response()
        }
    }
}

fn sync_failure(kind: &str, error: &cc_switch_lib::error::AppError) -> axum::response::Response {
    use axum::response::IntoResponse;
    (
        if matches!(error, cc_switch_lib::error::AppError::Conflict { .. }) {
            axum::http::StatusCode::CONFLICT
        } else {
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        },
        axum::Json(serde_json::json!({
            "error": format!("Record saved, but {kind} sync failed: {error}. Retry Sync."),
            "saved": true,
        })),
    )
        .into_response()
}

fn managed_idle(db: &cc_switch_lib::database::Database) -> Option<axum::response::Response> {
    use axum::response::IntoResponse;
    cc_switch_lib::managed_deletion::ensure_idle(db)
        .err()
        .map(|error| {
            (
                axum::http::StatusCode::CONFLICT,
                axum::Json(serde_json::json!({"error":error.to_string(), "recoveryRequired":true})),
            )
                .into_response()
        })
}

pub mod managed_items;
