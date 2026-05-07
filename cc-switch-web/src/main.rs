//! cc-switch-web
//!
//! Web Admin server for cc-switch running on port 5007

mod handlers;
mod proxy;
mod state;

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::{Redirect, Response},
    routing::{delete, get, post, put},
    Router,
};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, services::fs::ServeDir, trace::TraceLayer};

use cc_switch_lib::config::get_app_config_dir;
use cc_switch_lib::database::Database;
use cc_switch_lib::oauth::codex::CodexOAuthManager;
use cc_switch_lib::oauth::copilot::CopilotAuthManager;

use handlers::{auth, copilot_oauth, oauth, providers, settings, usage};
use state::AppState;

fn generate_token() -> String {
    use std::fmt::Write;
    let bytes: [u8; 32] = rand::random();
    bytes.iter().fold(String::new(), |mut acc, &b| {
        write!(&mut acc, "{:02x}", b).ok();
        acc
    })
}

fn migrate_legacy_oauth_file(config_dir: &Path, legacy_dir: &Path, file_name: &str) {
    if config_dir == legacy_dir {
        return;
    }

    let source = legacy_dir.join(file_name);
    let target = config_dir.join(file_name);
    if !source.exists() || target.exists() {
        return;
    }

    if let Some(parent) = target.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            log::warn!(
                "[OAuth] Failed to create config directory for legacy auth migration: {}",
                e
            );
            return;
        }
    }

    match std::fs::copy(&source, &target) {
        Ok(_) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600));
            }
            log::info!(
                "[OAuth] Migrated legacy auth store {} -> {}",
                source.display(),
                target.display()
            );
        }
        Err(e) => {
            log::warn!(
                "[OAuth] Failed to migrate legacy auth store {} -> {}: {}",
                source.display(),
                target.display(),
                e
            );
        }
    }
}

fn migrate_legacy_oauth_stores(config_dir: &Path) {
    let Some(legacy_dir) = dirs::config_dir().map(|dir| dir.join("cc-switch")) else {
        return;
    };

    migrate_legacy_oauth_file(config_dir, &legacy_dir, "codex_oauth_auth.json");
    migrate_legacy_oauth_file(config_dir, &legacy_dir, "copilot_auth.json");
}

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path();
    if path == "/" || path == "/api/auth/login" || path == "/health" || path.starts_with("/ui") {
        let response = next.run(request).await;
        if response.status().is_server_error() {
            log::error!(
                "[HTTP] {} {} failed with {}",
                method,
                uri,
                response.status()
            );
        }
        return response;
    }
    let valid = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|token| token == state.token)
        .unwrap_or(false);
    let response = if valid {
        next.run(request).await
    } else {
        Response::builder()
            .status(401)
            .body(Body::from(r#"{"error":"Unauthorized"}"#))
            .unwrap()
    };
    if response.status().is_server_error() {
        log::error!(
            "[HTTP] {} {} failed with {}",
            method,
            uri,
            response.status()
        );
    }
    response
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config_dir = get_app_config_dir();
    migrate_legacy_oauth_stores(&config_dir);
    let token = std::env::var("CC_SWITCH_ADMIN_TOKEN").unwrap_or_else(|_| generate_token());
    let proxy_port = std::env::var("CC_SWITCH_PROXY_PORT")
        .unwrap_or_else(|_| "15721".to_string())
        .parse()
        .unwrap_or(15721);

    let db = Database::init().expect("failed to initialize database");
    let codex_oauth = CodexOAuthManager::new(config_dir.clone());
    let copilot_oauth = CopilotAuthManager::new(config_dir.clone());

    // Load saved proxy config from database and apply to OAuth managers
    if let Ok(Some(proxy_config)) = db.get_proxy_config() {
        codex_oauth.set_proxy_config(&proxy_config).await;
        copilot_oauth.set_proxy_config(&proxy_config).await;
        if proxy_config.enabled {
            log::info!(
                "[Main] Loaded proxy config from database: {}://{}:{}",
                if proxy_config.proxy_type == cc_switch_lib::database::ProxyType::Http {
                    "http"
                } else {
                    "socks5"
                },
                proxy_config.host,
                proxy_config.port
            );
        }
    }

    // Find frontend dist directory
    // 1. Check CC_SWITCH_UI_DIR environment variable (for packaged releases)
    // 2. Fall back to exe path relative to dist (when packaged with frontend)
    // 3. Fall back to CARGO_MANIFEST_DIR (for local development)
    let ui_dist_dir = if let Ok(ui_dir) = std::env::var("CC_SWITCH_UI_DIR") {
        std::path::PathBuf::from(ui_dir)
    } else {
        let exe_path = std::env::current_exe()
            .unwrap_or_else(|_| std::path::PathBuf::from("cc-switch-web"))
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from("cc-switch-web"));
        let release_dist = exe_path.parent().unwrap_or_else(|| std::path::Path::new(".")).join("dist");
        if release_dist.exists() {
            release_dist
        } else {
            // Local development fallback
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("cc-switch-ui")
                .join("dist")
        }
    };

    let app_state = Arc::new(AppState {
        codex_oauth: Arc::new(codex_oauth),
        copilot_oauth: Arc::new(copilot_oauth),
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
        // Codex OAuth
        .route("/api/codex/oauth/status", get(oauth::codex_oauth_status))
        .route("/api/codex/oauth/start", post(oauth::codex_oauth_start))
        .route("/api/codex/oauth/poll", post(oauth::codex_oauth_poll))
        .route("/api/codex/oauth/remove", post(oauth::codex_oauth_remove))
        .route(
            "/api/codex/oauth/set-default",
            post(oauth::codex_oauth_set_default),
        )
        // Copilot OAuth
        .route(
            "/api/copilot/oauth/status",
            get(copilot_oauth::copilot_oauth_status),
        )
        .route(
            "/api/copilot/oauth/start",
            post(copilot_oauth::copilot_oauth_start),
        )
        .route(
            "/api/copilot/oauth/poll",
            post(copilot_oauth::copilot_oauth_poll),
        )
        .route(
            "/api/copilot/oauth/remove",
            post(copilot_oauth::copilot_oauth_remove),
        )
        .route(
            "/api/copilot/oauth/set-default",
            post(copilot_oauth::copilot_oauth_set_default),
        )
        // Copilot Usage
        .route("/api/copilot/usage", get(copilot_oauth::copilot_usage))
        // Proxy
        .route("/api/proxy/start", post(proxy::proxy_start))
        .route("/api/proxy/stop", post(proxy::proxy_stop))
        .route("/api/proxy/status", get(proxy::proxy_status))
        .route("/api/proxy/target", get(proxy::proxy_target))
        .route("/api/proxy/target", post(proxy::proxy_set_target))
        // Settings (Proxy Config)
        .route("/api/settings/proxy", get(settings::get_proxy_config))
        .route("/api/settings/proxy", put(settings::set_proxy_config))
        .route("/api/settings/proxy", delete(settings::delete_proxy_config))
        .route("/api/settings/proxy-port", get(settings::get_proxy_port))
        .route("/api/settings/proxy-port", put(settings::set_proxy_port))
        // Providers
        .route("/api/providers", get(providers::list_providers))
        .route("/api/providers", post(providers::save_provider))
        .route(
            "/api/providers/current",
            get(providers::get_current_provider),
        )
        .route("/api/providers/:id", get(providers::get_provider))
        .route("/api/providers/:id", put(providers::update_provider))
        .route("/api/providers/:id", delete(providers::delete_provider))
        .route(
            "/api/providers/:id/switch",
            post(providers::switch_provider),
        )
        // Usage
        .route("/api/usage/summary", get(usage::get_usage_summary))
        .route("/api/usage/trend", get(usage::get_usage_trend))
        .route("/api/usage/providers", get(usage::get_usage_providers))
        // Static files
        .nest_service("/ui", ServeDir::new(ui_dist_dir.clone()))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware,
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 5007));
    let listener = TcpListener::bind(addr).await.unwrap();
    log::info!("Server listening on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
