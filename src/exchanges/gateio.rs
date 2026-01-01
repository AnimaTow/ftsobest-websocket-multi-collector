use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData, BookData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType};

/// Gate.io WebSocket adapter
///
/// This adapter implements the `ExchangeAdapter` trait and encapsulates
/// all Gate.io–specific WebSocket behavior:
/// - subscription message formats
/// - symbol conversion
/// - message parsing
///
/// DESIGN PRINCIPLES:
/// - No business logic
/// - No storage logic
/// - No master communication
/// - Pure protocol translation only
///
/// All outgoing messages must conform to Gate.io WS v4 API.
/// All incoming messages are converted into the unified `MarketMessage` schema.
pub struct GateIoAdapter;

#[async_trait::async_trait]
impl ExchangeAdapter for GateIoAdapter {

    /// Returns the exchange identifier.
    ///
    /// CONTRACT:
    /// - Must match `exchange.name` in config.json
    /// - Must match the key used in util::symbol_* helpers
    fn name(&self) -> &'static str {
        "gateio"
    }

    /// Returns the WebSocket endpoint for Gate.io Spot API v4.
    fn ws_url(&self) -> &'static str {
        "wss://api.gateio.ws/ws/v4/"
    }

    /// Builds a subscription message for the given channel and pairs.
    ///
    /// This function translates the unified internal configuration
    /// into Gate.io–specific subscription payloads.
    ///
    /// SYMBOL HANDLING:
    /// - Input symbols are always in internal format: "BASE/QUOTE"
    /// - Gate.io requires "BASE_QUOTE"
    ///
    /// CHANNEL BEHAVIOR:
    /// - Trades: multiple pairs per connection allowed
    /// - OrderBooks: exactly one pair per connection
    ///
    /// TODO:
    /// - Add validation for empty `pairs`
    /// - Add support for unsubscribe events if required
    fn build_subscribe_message(
        &self,
        channel: ChannelType,
        pairs: &[String],
        config: &ExchangeConfig,
    ) -> Value {
        // Convert all symbols from internal format to Gate.io format
        let pairs: Vec<String> = pairs
            .iter()
            .map(|p| util::symbol_to_exchange(self.name(), p))
            .collect();

        match channel {
            // -------------------------------------------------
            // TRADES SUBSCRIPTION
            // -------------------------------------------------
            ChannelType::Trades => json!({
                "time": util::now_ms(),
                "channel": "spot.trades",
                "event": "subscribe",
                "payload": pairs
            }),

            // -------------------------------------------------
            // ORDER BOOK SUBSCRIPTION
            // -------------------------------------------------
            ChannelType::OrderBooks => {
                // Depth configuration (Gate.io supports fixed levels)
                let depth = config
                    .orderbook
                    .as_ref()
                    .map(|o| o.depth)
                    .unwrap_or(20);

                // Update interval configuration
                let interval = config
                    .orderbook
                    .as_ref()
                    .map(|o| format!("{}ms", o.update_interval_ms))
                    .unwrap_or_else(|| "1000ms".to_string());

                // Gate.io requires exactly one symbol per order book connection
                let symbol = util::symbol_to_exchange(self.name(), &pairs[0]);

                json!({
                    "time": util::now_ms(),
                    "channel": "spot.order_book",
                    "event": "subscribe",
                    "payload": [symbol, depth.to_string(), interval]
                })
            }
        }
    }

    /// Parses a raw WebSocket message into a unified `MarketMessage`.
    ///
    /// Messages that are not relevant (heartbeats, acks, etc.)
    /// return `None`.
    ///
    /// CONTRACT:
    /// - Only messages with `"event": "update"` are processed
    /// - Returned messages must be fully normalized
    ///
    /// ERROR HANDLING:
    /// - Any malformed message is silently ignored
    /// - No panics are allowed in this path
    fn parse_message(
        &self,
        raw: &str,
        exchange: &str,
    ) -> Option<MarketMessage> {
        let v: Value = serde_json::from_str(raw).ok()?;

        let channel = v.get("channel")?.as_str()?;
        let event = v.get("event")?.as_str()?;

        // Ignore non-update events (subscribe acks, system messages, etc.)
        if event != "update" {
            return None;
        }

        match channel {

            // -------------------------------------------------
            // TRADES
            // -------------------------------------------------
            "spot.trades" => {
                let r = &v["result"];

                Some(MarketMessage::Trade(TradeData {
                    exchange: exchange.to_string(),
                    symbol: util::symbol_from_exchange(
                        exchange,
                        r["currency_pair"].as_str()?
                    ),
                    timestamp: r["create_time_ms"]
                        .as_i64()
                        .unwrap_or_else(util::now_ms),
                    price: r["price"].as_str()?.to_string(),
                    amount: r["amount"].as_str()?.to_string(),
                    side: r["side"].as_str()?.to_string(),
                }))
            }

            // -------------------------------------------------
            // ORDER BOOK UPDATES
            // -------------------------------------------------
            "spot.order_book" => {
                let r = &v["result"];

                let asks = r["asks"]
                    .as_array()?
                    .iter()
                    .filter_map(|x| {
                        Some([
                            x.get(0)?.as_str()?.to_string(),
                            x.get(1)?.as_str()?.to_string(),
                        ])
                    })
                    .collect::<Vec<[String; 2]>>();

                let bids = r["bids"]
                    .as_array()?
                    .iter()
                    .filter_map(|x| {
                        Some([
                            x.get(0)?.as_str()?.to_string(),
                            x.get(1)?.as_str()?.to_string(),
                        ])
                    })
                    .collect::<Vec<[String; 2]>>();

                Some(MarketMessage::Book(BookData {
                    exchange: exchange.to_string(),
                    symbol: util::symbol_from_exchange(
                        exchange,
                        r["s"].as_str()?
                    ),
                    timestamp: r["t"]
                        .as_i64()
                        .unwrap_or_else(util::now_ms),
                    asks,
                    bids,
                }))
            }

            _ => None,
        }
    }
}
