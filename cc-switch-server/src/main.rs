//! cc-switch-server binary entrypoint.

use cc_switch_server::{
    print_start_banner, resolve_server_config, run_server, ServerLaunchOptions,
};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = resolve_server_config(ServerLaunchOptions::default());
    print_start_banner(&config);

    if let Err(err) = run_server(config).await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
