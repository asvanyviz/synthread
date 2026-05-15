//! Synthread headless binary — TUI + embedded WebUI
//!
//! Usage:
//!   synthread                   # auto-detect mode
//!   synthread --mode headless   # force headless
//!   synthread --mode headless --port 7700

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "synthread",
    version,
    about = "P2P framework with plugin system"
)]
struct Args {
    /// Operating mode: gui, headless, or auto (default)
    #[arg(long, default_value = "auto")]
    mode: String,

    /// HTTP API port for headless mode
    #[arg(long, default_value = "7700")]
    port: u16,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    tracing::info!(
        "Synthread v{} starting in {} mode",
        env!("CARGO_PKG_VERSION"),
        args.mode
    );

    match args.mode.as_str() {
        "headless" => run_headless(args.port).await,
        "gui" => {
            tracing::warn!("GUI mode not yet implemented — falling back to headless");
            run_headless(args.port).await;
        }
        _ => {
            // Auto-detect: if DISPLAY is set → GUI; otherwise headless
            if std::env::var("DISPLAY").is_ok() {
                tracing::warn!("GUI auto-detect: DISPLAY found, but GUI not implemented yet");
            }
            run_headless(args.port).await;
        }
    }
}

async fn run_headless(port: u16) {
    tracing::info!("Starting headless mode on port {}", port);
    // Phase 3: start TUI + embedded WebUI + API server
    tracing::info!("Headless mode stub — press Ctrl+C to exit");
    tokio::signal::ctrl_c().await.unwrap();
    tracing::info!("Shutting down");
}
