use serde_json::Value;
use crate::schema::MarketMessage;
use crate::config::ExchangeConfig;

/// Defines the supported logical data channels.
///
/// These are *logical* channels used by the collector.
/// Each exchange adapter is responsible for mapping them
/// to the exchange-specific WebSocket channels.
///
/// IMPORTANT:
/// - This enum must remain stable across the project
/// - Adding a new variant requires changes in:
///   - runner logic
///   - all exchange adapters
///
#[derive(Debug, Clone, Copy)]
pub enum ChannelType {
    /// Trade stream (individual executions)
    Trades,

    /// Orderbook stream (Level 2 updates, incremental)
    OrderBooks,
}

/// ExchangeAdapter is the core abstraction layer between:
/// - The generic collector runtime
/// - Exchange-specific WebSocket APIs
///
/// Each exchange implementation must:
/// - Define how to subscribe to channels
/// - Parse raw WebSocket messages
/// - Normalize data into MarketMessage
///
/// DESIGN GOALS:
/// - Zero exchange-specific logic outside adapters
/// - One adapter per exchange
/// - Uniform output format across all exchanges
///
/// THREAD SAFETY:
/// - Must be Send + Sync
/// - Adapter instances are shared across tasks
///
#[async_trait::async_trait]
pub trait ExchangeAdapter: Send + Sync {

    /// Returns the canonical exchange name.
    ///
    /// CONTRACT:
    /// - Must match `exchange.name` in configuration
    /// - Used for:
    ///   - Logging
    ///   - Symbol normalization
    ///   - Downstream routing
    ///
    /// EXAMPLES:
    /// - "gateio"
    /// - "binance"
    /// - "coinbase"
    ///
    fn name(&self) -> &'static str;

    /// Returns the WebSocket endpoint URL for this exchange.
    ///
    /// NOTES:
    /// - Must be a full WebSocket URL (wss://…)
    /// - No query parameters should be included here
    /// - Authentication (if required) is handled in messages
    ///
    fn ws_url(&self) -> &'static str;

    /// Builds a subscription message for a given channel.
    ///
    /// PARAMETERS:
    /// - `channel`: logical channel (Trades / OrderBooks)
    /// - `pairs`: symbols already converted to exchange format
    /// - `config`: exchange-specific configuration
    ///
    /// RETURNS:
    /// - A serde_json::Value representing the WS subscribe message
    ///
    /// RESPONSIBILITIES:
    /// - Apply exchange-specific payload format
    /// - Respect orderbook depth / interval config
    /// - Handle channel-specific quirks
    ///
    /// MUST NOT:
    /// - Perform network I/O
    /// - Mutate shared state
    ///
    /// TODO:
    /// - Support unsubscribe messages
    /// - Support dynamic resubscription
    ///
    fn build_subscribe_message(
        &self,
        channel: ChannelType,
        pairs: &[String],
        config: &ExchangeConfig,
    ) -> Value;

    /// Parses a raw WebSocket message into a MarketMessage.
    ///
    /// INPUT:
    /// - `raw`: raw text frame from WebSocket
    /// - `exchange_name`: adapter.name(), injected by runtime
    ///
    /// OUTPUT:
    /// - Some(MarketMessage) for valid market data
    /// - None for:
    ///   - Heartbeats
    ///   - Subscribe acknowledgements
    ///   - Errors
    ///   - Unsupported messages
    ///
    /// IMPORTANT:
    /// - This function must NEVER panic
    /// - Invalid messages must be ignored safely
    ///
    /// DATA NORMALIZATION:
    /// - Symbols must be converted to internal format (BASE/QUOTE)
    /// - Timestamps must be milliseconds (Unix epoch)
    /// - Numeric values must remain strings
    ///
    /// PERFORMANCE:
    /// - Called on every incoming WS message
    /// - Must be allocation-aware
    ///
    /// TODO:
    /// - Add structured error reporting (optional)
    ///
    fn parse_message(
        &self,
        raw: &str,
        exchange_name: &str,
    ) -> Option<MarketMessage>;
}
