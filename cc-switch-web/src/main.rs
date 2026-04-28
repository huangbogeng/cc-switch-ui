//! cc-switch-web
//!
//! Web Admin server for cc-switch running on port 5007

mod handlers;
mod proxy;
mod state;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::{IntoResponse, Response, Redirect},
    routing::{get, post, delete, put},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer, services::fs::ServeDir};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use cc_switch_lib::database::Database;
use cc_switch_lib::oauth::codex_oauth_auth::CodexOAuthManager;

use handlers::{auth, oauth, providers};
use state::AppState;

fn generate_token() -> String {
    use std::fmt::Write;
    let bytes: [u8; 32] = rand::random();
    bytes.iter().fold(String::new(), |mut acc, &b| {
        write!(&mut acc, "{:02x}", b).ok();
        acc
    })
}

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if path == "/" || path == "/api/auth/login" || path == "/health" || path.starts_with("/ui") {
        return next.run(request).await;
    }
    let valid = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|token| token == state.token)
        .unwrap_or(false);
    if valid { next.run(request).await } else {
        Response::builder()
            .status(401)
            .body(Body::from(r#"{"error":"Unauthorized"}"#))
            .unwrap()
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("cc-switch");
    let token = std::env::var("CC_SWITCH_ADMIN_TOKEN").unwrap_or_else(|_| generate_token());
    let proxy_port = std::env::var("CC_SWITCH_PROXY_PORT").unwrap_or_else(|_| "15721".to_string()).parse().unwrap_or(15721);

    let codex_oauth = CodexOAuthManager::new(config_dir.clone());
    let db = Database::init().expect("failed to initialize database");

    let ui_dist_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().join("cc-switch-ui").join("dist");

    let app_state = Arc::new(AppState {
        codex_oauth: Arc::new(codex_oauth),
        token: token.clone(),
        proxy_server: Arc::new(RwLock::new(None)),
        proxy_listen_port: proxy_port,
        db: Arc::new(db),
    });

    log::info!("========================================");
    log::info!("  CC-Switch Web Admin");
    log::info!("  Admin Token: {}", token);
    log::info!("  Web URL: http://localhost:5007/");
    log::info!("========================================");

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/", get(|| async { Redirect::permanent("/ui") }))
        // Auth
        .route("/api/auth/login", post(auth::login))
        // OAuth
        .route("/api/codex/oauth/status", get(oauth::codex_oauth_status))
        .route("/api/codex/oauth/start", post(oauth::codex_oauth_start))
        .route("/api/codex/oauth/poll", post(oauth::codex_oauth_poll))
        // Proxy
        .route("/api/proxy/start", post(proxy::proxy_start))
        .route("/api/proxy/stop", post(proxy::proxy_stop))
        .route("/api/proxy/status", get(proxy::proxy_status))
        // Providers
        .route("/api/providers", get(providers::list_providers))
        .route("/api/providers", post(providers::save_provider))
        .route("/api/providers/current", get(providers::get_current_provider))
        .route("/api/providers/:id", get(providers::get_provider))
        .route("/api/providers/:id", put(providers::update_provider))
        .route("/api/providers/:id", delete(providers::delete_provider))
        .route("/api/providers/:id/switch", post(providers::switch_provider))
        // Static files
        .nest_service("/ui", ServeDir::new(ui_dist_dir.clone()))
        .layer(axum::middleware::from_fn_with_state(app_state.clone(), auth_middleware))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 5007));
    let listener = TcpListener::bind(addr).await.unwrap();
    log::info!("Server listening on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
