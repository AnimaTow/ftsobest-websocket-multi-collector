use serde::{Serialize, Deserialize};

/// Central message enum used across the entire data pipeline.
///
/// This enum represents the unified message format exchanged between:
/// - Exchange collectors (Gate.io, Binance, Coinbase, etc.)
/// - The MasterSender
/// - Redis streams / storage backends
/// - Downstream processing systems
///
/// The `type` field is automatically added by serde and is used
/// for routing and deserialization on the master side
/// (e.g. "trade", "book", "ticker").
///
/// DESIGN NOTES:
/// - This enum is intentionally stable and version-agnostic.
/// - Any schema changes here affect the entire ingestion pipeline.
///
/// TODO:
/// - Consider introducing a `version` field for long-term schema evolution.
///
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MarketMessage {
    Trade(TradeData),
    Book(BookData),
    Ticker(TickerData),
}

// ------------------------------------------------------------
// Trade message
// ------------------------------------------------------------
//
// Represents a single executed trade.
//
// Used by exchanges such as:
// - Gate.io (spot.trades)
// - Binance (trade)
// - Coinbase (match)
//
// This structure is normalized across all exchanges.
//
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeData {
    /// Exchange identifier (e.g. "gateio", "binance", "coinbase")
    pub exchange: String,

    /// Trading pair in normalized internal format
    /// Example: "BTC/USDT", "ETH/USD"
    pub symbol: String,

    /// Trade timestamp in milliseconds since Unix epoch
    pub timestamp: i64,

    /// Trade price as string
    ///
    /// DESIGN DECISION:
    /// Stored as string to avoid floating-point precision issues.
    pub price: String,

    /// Trade amount / volume as string
    pub amount: String,

    /// Trade side: "buy" or "sell"
    pub side: String,
}

// ------------------------------------------------------------
// Orderbook message (Level 2)
// ------------------------------------------------------------
//
// Represents an incremental orderbook update (not a snapshot).
//
// Compatible with:
// - Gate.io order book updates
// - Binance depth updates
// - Coinbase level2 updates
//
// IMPORTANT:
// - This structure represents the *current view* after update,
//   not a diff format.
// - Deduplication is handled downstream (master / Redis).
//
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookData {
    /// Exchange identifier
    pub exchange: String,

    /// Trading pair in normalized internal format
    pub symbol: String,

    /// Timestamp of the update in milliseconds
    pub timestamp: i64,

    /// Ask side levels: [price, amount]
    ///
    /// Sorted ascending by price (best ask first).
    pub asks: Vec<[String; 2]>,

    /// Bid side levels: [price, amount]
    ///
    /// Sorted descending by price (best bid first).
    pub bids: Vec<[String; 2]>,
}

// ------------------------------------------------------------
// Ticker message (optional / reserved)
// ------------------------------------------------------------
//
// Currently not actively used by the collector pipeline,
// but intentionally kept in the schema for:
//
// - Future feature expansion
// - Simple collector extensions
// - API compatibility with external consumers
//
// TODO:
// - Implement ticker collectors where available.
// - Define update frequency guarantees.
//
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TickerData {
    /// Exchange identifier
    pub exchange: String,

    /// Trading pair in normalized format
    pub symbol: String,

    /// Timestamp in milliseconds
    pub timestamp: i64,

    /// Best bid price
    pub bid: Option<String>,

    /// Best ask price
    pub ask: Option<String>,

    /// Last traded price
    pub last: Option<String>,

    /// 24h traded volume
    pub vol_24h: Option<String>,
}
