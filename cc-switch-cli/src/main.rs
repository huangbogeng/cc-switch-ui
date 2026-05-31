use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use cc_switch_lib::config::get_app_config_dir;
use cc_switch_server::{
    print_start_banner, resolve_server_config, run_server, ServerConfig, ServerLaunchOptions,
};

#[derive(Debug, Parser)]
#[command(
    name = "cc-switch-ui",
    bin_name = "cc-switch-ui",
    version,
    about = "CC Switch UI server and CLI",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Start server (backend + hosted frontend)
    Start {
        /// Bind host (defaults to saved config or 0.0.0.0)
        #[arg(long)]
        host: Option<String>,
        /// Bind port for admin/API server (defaults to saved config or 5007)
        #[arg(long)]
        port: Option<u16>,
        /// Proxy listen port (defaults to saved config or 15721)
        #[arg(long)]
        proxy_port: Option<u16>,
        /// Fixed admin token; when omitted uses env or generates one
        #[arg(long)]
        admin_token: Option<String>,
        /// Optional frontend dist directory override
        #[arg(long)]
        ui_dir: Option<PathBuf>,
    },
    /// Check service health
    Status {
        /// Host used for health check (defaults to 127.0.0.1)
        #[arg(long)]
        host: Option<String>,
        /// Port used for health check (defaults to saved config or 5007)
        #[arg(long)]
        port: Option<u16>,
    },
    /// Print version information
    Version,
    /// Diagnose installation, path, and database permissions
    Doctor,
    /// Stop server using pid file
    Stop,
    /// Remove all persisted provider credentials and database
    Clean {
        /// Also clean the live Claude Code config (removes proxy base URL)
        #[arg(long)]
        live: bool,
    },
    /// Manage persisted CLI defaults
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommands {
    /// Show current saved CLI defaults
    Get,
    /// Update saved CLI defaults
    Set {
        /// Default host used by `start`
        #[arg(long)]
        host: Option<String>,
        /// Default port used by `start`
        #[arg(long)]
        port: Option<u16>,
        /// Default proxy port used by `start`
        #[arg(long)]
        proxy_port: Option<u16>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CliConfig {
    host: String,
    port: u16,
    proxy_port: u16,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 5007,
            proxy_port: 15721,
        }
    }
}

fn cli_config_path(config_dir: &Path) -> PathBuf {
    config_dir.join("cli.json")
}

fn pid_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join("cc-switch-server.pid")
}

fn load_cli_config(config_dir: &Path) -> CliConfig {
    let path = cli_config_path(config_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str::<CliConfig>(&content).unwrap_or_default(),
        Err(_) => CliConfig::default(),
    }
}

fn save_cli_config(config_dir: &Path, config: &CliConfig) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| format!("create dir failed: {e}"))?;
    let path = cli_config_path(config_dir);
    let json =
        serde_json::to_string_pretty(config).map_err(|e| format!("json encode failed: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("write {} failed: {e}", path.display()))
}

fn write_pid_file(config_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| format!("create dir failed: {e}"))?;
    let path = pid_file_path(config_dir);
    std::fs::write(path, std::process::id().to_string())
        .map_err(|e| format!("write pid failed: {e}"))
}

fn read_pid_file(config_dir: &Path) -> Option<u32> {
    let path = pid_file_path(config_dir);
    let content = std::fs::read_to_string(path).ok()?;
    content.trim().parse::<u32>().ok()
}

fn remove_pid_file(config_dir: &Path) {
    let _ = std::fs::remove_file(pid_file_path(config_dir));
}

fn compose_server_options(
    host: Option<String>,
    port: Option<u16>,
    proxy_port: Option<u16>,
    admin_token: Option<String>,
    ui_dir: Option<PathBuf>,
) -> ServerLaunchOptions {
    let config_dir = get_app_config_dir();
    let cli = load_cli_config(&config_dir);

    ServerLaunchOptions {
        host: host.unwrap_or(cli.host),
        port: port.unwrap_or(cli.port),
        proxy_port: proxy_port.or(Some(cli.proxy_port)),
        admin_token,
        ui_dir,
    }
}

async fn run_status(host: Option<String>, port: Option<u16>) -> i32 {
    let config_dir = get_app_config_dir();
    let cli = load_cli_config(&config_dir);

    let host = host.unwrap_or_else(|| "127.0.0.1".to_string());
    let port = port.unwrap_or(cli.port);

    let url = format!("http://{}:{}/health", host, port);
    match reqwest::get(&url).await {
        Ok(resp) if resp.status() == StatusCode::OK => {
            println!("status: running");
            println!("health: {}", url);
            0
        }
        Ok(resp) => {
            println!("status: degraded");
            println!("health: {}", url);
            println!("http_status: {}", resp.status());
            1
        }
        Err(err) => {
            println!("status: stopped");
            println!("health: {}", url);
            println!("error: {}", err);
            1
        }
    }
}

fn stop_pid(pid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        let status = std::process::Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status()
            .map_err(|e| format!("failed to run kill: {e}"))?;
        if status.success() {
            return Ok(());
        }
        Err(format!("kill exited with status: {status}"))
    }

    #[cfg(windows)]
    {
        let status = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status()
            .map_err(|e| format!("failed to run taskkill: {e}"))?;
        if status.success() {
            return Ok(());
        }
        Err(format!("taskkill exited with status: {status}"))
    }
}

fn run_stop() -> i32 {
    let config_dir = get_app_config_dir();
    let Some(pid) = read_pid_file(&config_dir) else {
        println!(
            "stop: no pid file found ({})",
            pid_file_path(&config_dir).display()
        );
        return 1;
    };

    match stop_pid(pid) {
        Ok(()) => {
            println!("stop: sent terminate signal to pid {}", pid);
            remove_pid_file(&config_dir);
            0
        }
        Err(err) => {
            println!("stop: failed to stop pid {}: {}", pid, err);
            1
        }
    }
}

fn path_command_hits(name: &str) -> Vec<PathBuf> {
    let mut hits = Vec::new();
    let path_var = std::env::var_os("PATH");
    if let Some(path_value) = path_var {
        for dir in std::env::split_paths(&path_value) {
            let candidate = dir.join(name);
            if candidate.exists() {
                hits.push(candidate);
            }
        }
    }
    hits
}

fn can_write(path: &Path) -> bool {
    let probe = path.join(".cc-switch-write-probe");
    let result = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&probe)
        .and_then(|_| std::fs::remove_file(&probe));
    result.is_ok()
}

fn run_clean(clean_live: bool) -> i32 {
    let config_dir = get_app_config_dir();
    let db_path = config_dir.join("cc-switch.db");
    let settings_path = config_dir.join("settings.json");
    let cli_config_path = config_dir.join("cli.json");
    let pid_path = config_dir.join("cc-switch-server.pid");
    let mut removed = 0u32;
    let mut errors = 0u32;

    let mut remove = |path: &Path, label: &str| {
        if path.exists() {
            match std::fs::remove_file(path) {
                Ok(()) => {
                    println!("  removed: {}", path.display());
                    removed += 1;
                }
                Err(e) => {
                    eprintln!("  ERROR removing {} ({}): {}", label, path.display(), e);
                    errors += 1;
                }
            }
        } else {
            println!("  skipped (not found): {}", path.display());
        }
    };

    println!("clean: removing cc-switch-ui local data");
    println!("  config_dir: {}", config_dir.display());

    remove(&db_path, "database");
    remove(&settings_path, "device settings");
    remove(&cli_config_path, "CLI config");
    remove(&pid_path, "PID file");

    if clean_live {
        let live_path = cc_switch_lib::config::get_claude_settings_path();
        if live_path.exists() {
            match std::fs::read_to_string(&live_path) {
                Ok(raw) => {
                    let mut value: serde_json::Value =
                        serde_json::from_str(&raw).unwrap_or_default();
                    let env = value
                        .get_mut("env")
                        .and_then(|v| v.as_object_mut());
                    if let Some(env) = env {
                        // Only strip proxy-routing fields; keep legitimate user config.
                        let base_url = env
                            .get("ANTHROPIC_BASE_URL")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if base_url.starts_with("http://127.0.0.1:") {
                            env.remove("ANTHROPIC_BASE_URL");
                            env.remove("ANTHROPIC_AUTH_TOKEN");
                            env.remove("ANTHROPIC_API_KEY");
                            println!("  stripped proxy fields from live config");
                        } else {
                            println!(
                                "  live config base URL is not proxy — left untouched"
                            );
                        }
                        match cc_switch_lib::config::write_json_file(&live_path, &value) {
                            Ok(()) => println!("  updated: {}", live_path.display()),
                            Err(e) => {
                                eprintln!("  ERROR writing live config: {}", e);
                                errors += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  ERROR reading live config ({}): {}", live_path.display(), e);
                    errors += 1;
                }
            }
        } else {
            println!("  skipped live config (not found): {}", live_path.display());
        }
    }

    println!();
    if errors == 0 {
        println!("clean: done ({removed} file(s) removed)");
        0
    } else {
        eprintln!("clean: done with {errors} error(s) ({removed} file(s) removed)");
        1
    }
}

fn run_doctor() -> i32 {
    let mut has_error = false;

    let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("unknown"));
    let config_dir = get_app_config_dir();
    let db_path = config_dir.join("cc-switch.db");

    println!("doctor: cc-switch-ui");
    println!("version: {}", env!("CARGO_PKG_VERSION"));
    println!("exe: {}", current_exe.display());
    println!("config_dir: {}", config_dir.display());
    println!("db_path: {}", db_path.display());

    let ui_hits = path_command_hits("cc-switch-ui");
    if ui_hits.is_empty() {
        println!("path_check: WARN no cc-switch-ui found in PATH");
    } else {
        println!("path_check: found {} candidate(s)", ui_hits.len());
        for (idx, hit) in ui_hits.iter().enumerate() {
            println!("  {}. {}", idx + 1, hit.display());
        }
        if ui_hits.len() > 1 {
            println!(
                "path_check_hint: multiple candidates detected; earlier PATH entry may shadow newer install"
            );
        }
    }

    let legacy_dir = dirs::home_dir().map(|d| d.join(".cc-switch"));
    if let Some(legacy) = legacy_dir {
        if legacy.join("cc-switch-web").exists() || legacy.join("cc-switch-ui").exists() {
            println!(
                "path_check_hint: legacy binaries found under {} (consider removing old launchers)",
                legacy.display()
            );
        }
    }

    if !config_dir.exists() {
        println!("config_dir_check: WARN directory does not exist yet");
    } else if can_write(&config_dir) {
        println!("config_dir_check: OK writable");
    } else {
        println!("config_dir_check: ERROR not writable");
        has_error = true;
    }

    if let Some(parent) = db_path.parent() {
        if parent.exists() && !can_write(parent) {
            println!("db_parent_check: ERROR not writable ({})", parent.display());
            has_error = true;
        } else {
            println!("db_parent_check: OK writable ({})", parent.display());
        }
    }

    if db_path.exists() {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&db_path)
        {
            Ok(_) => println!("db_file_check: OK read/write"),
            Err(err) => {
                println!("db_file_check: ERROR cannot open read/write: {}", err);
                has_error = true;
            }
        }
    } else {
        println!("db_file_check: WARN database file not created yet");
    }

    if has_error {
        1
    } else {
        0
    }
}

fn run_config(command: ConfigCommands) -> i32 {
    let config_dir = get_app_config_dir();
    match command {
        ConfigCommands::Get => {
            let cfg = load_cli_config(&config_dir);
            println!("host: {}", cfg.host);
            println!("port: {}", cfg.port);
            println!("proxy_port: {}", cfg.proxy_port);
            println!("config_file: {}", cli_config_path(&config_dir).display());
            0
        }
        ConfigCommands::Set {
            host,
            port,
            proxy_port,
        } => {
            let mut cfg = load_cli_config(&config_dir);
            if let Some(v) = host {
                cfg.host = v;
            }
            if let Some(v) = port {
                cfg.port = v;
            }
            if let Some(v) = proxy_port {
                cfg.proxy_port = v;
            }
            match save_cli_config(&config_dir, &cfg) {
                Ok(()) => {
                    println!("config: updated");
                    println!("host: {}", cfg.host);
                    println!("port: {}", cfg.port);
                    println!("proxy_port: {}", cfg.proxy_port);
                    println!("config_file: {}", cli_config_path(&config_dir).display());
                    0
                }
                Err(err) => {
                    println!("config: failed to save: {}", err);
                    1
                }
            }
        }
    }
}

async fn start_and_run(opts: ServerLaunchOptions) -> i32 {
    let config_dir = get_app_config_dir();
    if let Err(err) = write_pid_file(&config_dir) {
        eprintln!("warn: failed to write pid file: {err}");
    }

    let resolved: ServerConfig = resolve_server_config(opts);
    print_start_banner(&resolved);

    match run_server(resolved).await {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    let code = match cli.command {
        Some(Commands::Start {
            host,
            port,
            proxy_port,
            admin_token,
            ui_dir,
        }) => {
            let opts = compose_server_options(host, port, proxy_port, admin_token, ui_dir);
            start_and_run(opts).await
        }
        Some(Commands::Status { host, port }) => run_status(host, port).await,
        Some(Commands::Version) => {
            println!("cc-switch-ui {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Some(Commands::Doctor) => run_doctor(),
        Some(Commands::Stop) => run_stop(),
        Some(Commands::Clean { live }) => run_clean(live),
        Some(Commands::Config { command }) => run_config(command),
        None => start_and_run(compose_server_options(None, None, None, None, None)).await,
    };

    std::process::exit(code);
}
