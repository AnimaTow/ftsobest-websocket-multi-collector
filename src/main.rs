// ------------------------------------------------------------
// Module declarations
// ------------------------------------------------------------
//
// Each module represents a well-defined responsibility:
//
// - config:        Configuration structs loaded from JSON
// - schema:        Strongly typed market message definitions
// - util:          Shared helper utilities (time, symbol handling, etc.)
// - exchanges:     Exchange adapters and adapter registry
// - master_sender: WebSocket client pool for sending data to the master
// - collector:     Exchange runtime (connection + subscription logic)
//
mod config;
mod schema;
mod util;
mod exchanges;
mod master_sender;
mod collector;

// ------------------------------------------------------------
// External dependencies
// ------------------------------------------------------------

use rustls::crypto::{CryptoProvider, ring};

use config::Config;
use exchanges::get_adapter;
use collector::runner::run_exchange;
use master_sender::MasterPool;

use std::fs;

// ------------------------------------------------------------
// Application entry point
// ------------------------------------------------------------
//
// This is the main runtime for the multi-exchange WebSocket collector.
//
// Responsibilities:
// - Initialize cryptography backend (rustls)
// - Load configuration
// - Create and manage the MasterPool
// - Start enabled exchange collectors
// - Keep the process alive indefinitely
//
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --------------------------------------------------------
    // IMPORTANT:
    // rustls >= 0.23 requires an explicit CryptoProvider
    // installation. This must be executed exactly once and
    // as early as possible in the process lifecycle.
    //
    // Using the `ring` provider for performance and stability.
    // --------------------------------------------------------
    CryptoProvider::install_default(ring::default_provider())
        .expect("failed to install rustls CryptoProvider");

    // --------------------------------------------------------
    // Load configuration from disk
    //
    // NOTE:
    // - The config file contains sensitive data (master key).
    // - It must not be committed to version control.
    // --------------------------------------------------------
    let config: Config = load_config("config.json")?;

    // --------------------------------------------------------
    // Initialize the MasterPool
    //
    // The MasterPool manages multiple persistent WebSocket
    // connections to the master ingestion service.
    //
    // Features:
    // - Automatic reconnect
    // - Load balancing
    // - Backpressure handling
    // - Optional demo mode (no data sent)
    // --------------------------------------------------------
    let master = MasterPool::new(
        config.master.url.clone(),
        config.master.key.clone(),
        config.debug
            .as_ref()
            .map_or(false, |d| d.log.unwrap_or(false)),
        config.master.connections,
        config.master.demo.unwrap_or(false),
    ).await;

    // --------------------------------------------------------
    // Start all enabled exchange collectors
    //
    // Each exchange:
    // - Resolves its adapter implementation
    // - Spawns independent WebSocket runtimes
    // - Shares the same MasterPool
    //
    // Unsupported exchanges are skipped gracefully.
    // --------------------------------------------------------
    for exchange_cfg in config.exchanges.iter().filter(|e| e.enabled) {
        let Some(adapter) = get_adapter(&exchange_cfg.name) else {
            eprintln!("Exchange '{}' is not supported", exchange_cfg.name);
            continue;
        };

        println!("Starting {} collector", exchange_cfg.name);

        run_exchange(
            adapter,
            exchange_cfg.clone(),
            master.clone(),
        ).await?;
    }

    // --------------------------------------------------------
    // Keep the process alive forever
    //
    // All collectors run in background tasks.
    // This future never resolves.
    // --------------------------------------------------------
    futures_util::future::pending::<()>().await;

    Ok(())
}

// ------------------------------------------------------------
// Configuration loader
// ------------------------------------------------------------
//
// Reads a JSON configuration file from disk and deserializes
// it into the strongly typed `Config` structure.
//
// TODO:
// - Support loading from environment variables
// - Support CLI override (e.g. --config path)
// - Validate config semantics (e.g. empty pair lists)
//
fn load_config(path: &str) -> anyhow::Result<Config> {
    let data = fs::read_to_string(path)?;
    let cfg = serde_json::from_str(&data)?;
    Ok(cfg)
}
