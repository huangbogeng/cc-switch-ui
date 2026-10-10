pub mod auth;
pub mod copilot_oauth;
pub mod mcp;
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
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        axum::Json(serde_json::json!({
            "error": format!("Record saved, but {kind} sync failed: {error}. Retry Sync."),
            "saved": true,
        })),
    )
        .into_response()
}
