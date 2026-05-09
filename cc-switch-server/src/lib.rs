//! cc-switch-server library
//!
//! Exposes server startup APIs used by both the server binary and cc-switch CLI.

mod handlers;
mod proxy;
mod state;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
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

#[derive(Debug, Clone)]
pub struct ServerLaunchOptions {
    pub host: String,
    pub port: u16,
    pub proxy_port: Option<u16>,
    pub admin_token: Option<String>,
    pub ui_dir: Option<PathBuf>,
}

impl Default for ServerLaunchOptions {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 5007,
            proxy_port: None,
            admin_token: None,
            ui_dir: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub proxy_port: u16,
    pub token: String,
    pub ui_dist_dir: PathBuf,
    pub config_dir: PathBuf,
}

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

fn resolve_ui_dist_dir(explicit_ui_dir: Option<PathBuf>) -> PathBuf {
    if let Some(ui_dir) = explicit_ui_dir {
        return ui_dir;
    }

    if let Ok(ui_dir) = std::env::var("CC_SWITCH_UI_DIR") {
        return PathBuf::from(ui_dir);
    }

    let exe_path = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("cc-switch-server"))
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("cc-switch-server"));
    let release_dist = exe_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("dist");

    if release_dist.exists() {
        release_dist
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("cc-switch-ui")
            .join("dist")
    }
}

pub fn resolve_server_config(options: ServerLaunchOptions) -> ServerConfig {
    let config_dir = get_app_config_dir();
    migrate_legacy_oauth_stores(&config_dir);

    let token = options
        .admin_token
        .or_else(|| std::env::var("CC_SWITCH_ADMIN_TOKEN").ok())
        .unwrap_or_else(generate_token);

    let proxy_port = options.proxy_port.unwrap_or_else(|| {
        std::env::var("CC_SWITCH_PROXY_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(15721)
    });

    let ui_dist_dir = resolve_ui_dist_dir(options.ui_dir);

    ServerConfig {
        host: options.host,
        port: options.port,
        proxy_port,
        token,
        ui_dist_dir,
        config_dir,
    }
}

pub fn print_start_banner(config: &ServerConfig) {
    let external_url = format!("http://{}:{}", config.host, config.port);
    let local_url = format!("http://127.0.0.1:{}", config.port);
    let ui_url = format!("{}/ui", local_url);
    let proxy_url = format!("http://127.0.0.1:{}", config.proxy_port);
    let db_path = config.config_dir.join("cc-switch.db");
    let ui_status = if config.ui_dist_dir.exists() {
        "OK"
    } else {
        "MISSING"
    };

    println!();
    println!("========================================");
    println!("  CC Switch UI");
    println!("========================================");
    println!("  Version:       {}", env!("CARGO_PKG_VERSION"));
    println!("  Server Bind:   {}", external_url);
    println!("  Web UI:        {}", ui_url);
    println!("  Proxy:         {}", proxy_url);
    println!("  Data Dir:      {}", config.config_dir.display());
    println!("  Database:      {}", db_path.display());
    println!(
        "  Frontend:      {} ({})",
        ui_status,
        config.ui_dist_dir.display()
    );
    println!("  Admin Token:   {}", config.token);
    if ui_status == "MISSING" {
        println!("  Hint:          build frontend assets first (cd cc-switch-ui && npm run build)");
    }
    println!("========================================");
    println!();
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

pub async fn run_server(config: ServerConfig) -> Result<(), String> {
    let db = Database::init().map_err(|e| format!("failed to initialize database: {e}"))?;
    let codex_oauth = CodexOAuthManager::new(config.config_dir.clone());
    let copilot_oauth = CopilotAuthManager::new(config.config_dir.clone());

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

    let app_state = Arc::new(AppState {
        codex_oauth: Arc::new(codex_oauth),
        copilot_oauth: Arc::new(copilot_oauth),
        token: config.token.clone(),
        proxy_server: Arc::new(RwLock::new(None)),
        proxy_listen_port: config.proxy_port,
        db: Arc::new(db),
    });

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/", get(|| async { Redirect::permanent("/ui") }))
        .route("/api/auth/login", post(auth::login))
        .route("/api/codex/oauth/status", get(oauth::codex_oauth_status))
        .route("/api/codex/oauth/start", post(oauth::codex_oauth_start))
        .route("/api/codex/oauth/poll", post(oauth::codex_oauth_poll))
        .route("/api/codex/oauth/remove", post(oauth::codex_oauth_remove))
        .route(
            "/api/codex/oauth/set-default",
            post(oauth::codex_oauth_set_default),
        )
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
        .route("/api/copilot/usage", get(copilot_oauth::copilot_usage))
        .route("/api/proxy/start", post(proxy::proxy_start))
        .route("/api/proxy/stop", post(proxy::proxy_stop))
        .route("/api/proxy/status", get(proxy::proxy_status))
        .route("/api/proxy/target", get(proxy::proxy_target))
        .route("/api/proxy/target", post(proxy::proxy_set_target))
        .route("/api/settings/proxy", get(settings::get_proxy_config))
        .route("/api/settings/proxy", put(settings::set_proxy_config))
        .route("/api/settings/proxy", delete(settings::delete_proxy_config))
        .route("/api/settings/proxy-port", get(settings::get_proxy_port))
        .route("/api/settings/proxy-port", put(settings::set_proxy_port))
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
        .route("/api/usage/summary", get(usage::get_usage_summary))
        .route(
            "/api/usage/request-logs",
            get(usage::get_proxy_request_logs),
        )
        .nest_service("/ui", ServeDir::new(config.ui_dist_dir.clone()))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware,
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let addr = format!("{}:{}", config.host, config.port)
        .parse::<SocketAddr>()
        .map_err(|e| format!("invalid host/port: {e}"))?;
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind failed: {e}"))?;
    log::info!("Server listening on http://{}", addr);
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("server failed: {e}"))
}
