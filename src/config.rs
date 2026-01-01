use serde::Deserialize;

// ------------------------------------------------------------
// Root configuration
// ------------------------------------------------------------
//
// This is the top-level configuration structure loaded from
// `config.json`.
//
// It defines:
// - Master connection settings
// - Enabled exchanges and their parameters
// - Optional debug configuration
//
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// Configuration for the master ingestion service
    pub master: MasterConfig,

    /// List of exchange configurations
    pub exchanges: Vec<ExchangeConfig>,

    /// Optional debug configuration
    pub debug: Option<DebugConfig>,
}

// ------------------------------------------------------------
// Master configuration
// ------------------------------------------------------------
//
// Defines how this collector connects to the master service.
//
// Notes:
// - The `key` is security-sensitive and must never be committed.
// - `connections` controls parallel WebSocket connections.
// - `demo` disables sending data to the master (local testing).
//
#[derive(Debug, Deserialize, Clone)]
pub struct MasterConfig {
    /// WebSocket URL of the master service
    pub url: String,

    /// Number of parallel WebSocket connections to the master
    pub connections: usize,

    /// Authentication key used during login
    /// (sent as: key=XYZ&role=collector)
    pub key: String,

    /// Demo mode flag (no data sent, only logged)
    pub demo: Option<bool>,
}

// ------------------------------------------------------------
// Exchange configuration
// ------------------------------------------------------------
//
// Configuration for a single exchange instance.
//
// Each exchange runs independently and may:
// - Subscribe to trades
// - Subscribe to orderbooks
// - Use different chunking strategies
//
#[derive(Debug, Deserialize, Clone)]
pub struct ExchangeConfig {
    /// Exchange identifier (e.g. "gateio", "binance", "okx")
    pub name: String,

    /// Enables or disables this exchange at runtime
    pub enabled: bool,

    /// Trading pairs to subscribe to
    pub pairs: ExchangePairs,

    /// Chunking configuration for WebSocket connections
    pub chunking: ExchangeChunking,

    /// Optional orderbook-specific configuration
    pub orderbook: Option<OrderbookConfig>,
}

// ------------------------------------------------------------
// Exchange pair lists
// ------------------------------------------------------------
//
// Defines which market symbols should be subscribed.
//
// IMPORTANT:
// - Symbols are expected in normalized form: BASE/QUOTE
//   Example: "BTC/USDT"
// - Each exchange adapter is responsible for converting
//   symbols into exchange-specific formats if required.
//
#[derive(Debug, Deserialize, Clone)]
pub struct ExchangePairs {
    /// Trading pairs for trade subscriptions
    pub trades: Vec<String>,

    /// Trading pairs for orderbook subscriptions
    pub orderbooks: Vec<String>,
}

// ------------------------------------------------------------
// Chunking configuration
// ------------------------------------------------------------
//
// Controls how many pairs are grouped into a single WebSocket
// connection.
//
// Purpose:
// - Prevents oversized subscriptions
// - Improves fault isolation
// - Allows fine-grained scaling
//
#[derive(Debug, Deserialize, Clone)]
pub struct ExchangeChunking {
    /// Number of trade pairs per WebSocket connection
    pub trades_per_connection: usize,

    /// Number of orderbook pairs per WebSocket connection
    ///
    /// NOTE:
    /// Currently unused for Gate.io, which requires one
    /// orderbook subscription per connection.
    #[allow(dead_code)]
    pub orderbooks_per_connection: usize,
}

// ------------------------------------------------------------
// Orderbook configuration
// ------------------------------------------------------------
//
// Exchange-specific orderbook parameters.
//
// Typical use cases:
// - Control depth for bandwidth optimization
// - Control update interval for CPU/load balancing
//
#[derive(Debug, Deserialize, Clone)]
pub struct OrderbookConfig {
    /// Orderbook depth (e.g. 20, 50, 100)
    pub depth: usize,

    /// Update interval in milliseconds
    pub update_interval_ms: u64,
}

// ------------------------------------------------------------
// Debug configuration
// ------------------------------------------------------------
//
// Optional debug flags used during development and testing.
//
#[derive(Debug, Deserialize, Clone)]
pub struct DebugConfig {
    /// Enables raw WebSocket message logging
    ///
    /// TODO:
    /// Implement raw message passthrough in collectors
    #[allow(dead_code)]
    pub raw: Option<bool>,

    /// Enables structured debug logging
    pub log: Option<bool>,
}
