use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData, BookData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType, ParseResult};

/// Coinbase WebSocket adapter
///
/// Coinbase Advanced Trade WS:
/// wss://ws-feed.exchange.coinbase.com
///
/// Channels:
/// - matches  → trades
/// - level2   → order book deltas
pub struct CoinbaseAdapter;

#[async_trait::async_trait]
impl ExchangeAdapter for CoinbaseAdapter {

    fn name(&self) -> &'static str {
        "coinbase"
    }

    fn ws_url(&self) -> &'static str {
        "wss://ws-feed.exchange.coinbase.com"
    }

    fn build_subscribe_message(
        &self,
        channel: ChannelType,
        pairs: &[String],
        _config: &ExchangeConfig,
    ) -> Value {

        let product_ids: Vec<String> = pairs
            .iter()
            .map(|p| util::symbol_to_exchange(self.name(), p))
            .collect();

        match channel {
            ChannelType::Trades => json!({
                "type": "subscribe",
                "product_ids": product_ids,
                "channels": ["matches"]
            }),

            ChannelType::OrderBooks => json!({
                "type": "subscribe",
                "product_ids": product_ids,
                "channels": ["level2"]
            }),
        }
    }

    fn parse_message(
        &self,
        raw: &str,
        exchange: &str,
    ) -> ParseResult {

        let v: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return ParseResult::Error,
        };

        let msg_type = match v.get("type").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => return ParseResult::Control,
        };

        match msg_type {

            // --------------------------------------------------
            // TRADES
            // --------------------------------------------------
            "match" => {
                let msg = MarketMessage::Trade(TradeData {
                    exchange: exchange.to_string(),
                    symbol: util::symbol_from_exchange(
                        exchange,
                        v.get("product_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                    ),
                    timestamp: util::now_ms(),
                    price: v.get("price")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0")
                        .to_string(),
                    amount: v.get("size")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0")
                        .to_string(),
                    side: v.get("side")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                });

                ParseResult::Market(msg)
            }

            // --------------------------------------------------
            // ORDER BOOK (L2 delta)
            // --------------------------------------------------
            "l2update" => {
                let product_id = match v.get("product_id").and_then(|v| v.as_str()) {
                    Some(p) => p,
                    None => return ParseResult::Error,
                };

                let changes = match v.get("changes").and_then(|v| v.as_array()) {
                    Some(c) => c,
                    None => return ParseResult::Control,
                };

                let mut bids = Vec::new();
                let mut asks = Vec::new();

                for c in changes {
                    let side = match c.get(0).and_then(|v| v.as_str()) {
                        Some(s) => s,
                        None => continue,
                    };

                    let price = match c.get(1).and_then(|v| v.as_str()) {
                        Some(p) => p.to_string(),
                        None => continue,
                    };

                    let qty = match c.get(2).and_then(|v| v.as_str()) {
                        Some(q) => q.to_string(),
                        None => continue,
                    };

                    if qty == "0" {
                        continue;
                    }

                    match side {
                        "buy"  => bids.push([price, qty]),
                        "sell" => asks.push([price, qty]),
                        _ => {}
                    }
                }

                let msg = MarketMessage::Book(BookData {
                    exchange: exchange.to_string(),
                    symbol: util::symbol_from_exchange(exchange, product_id),
                    timestamp: util::now_ms(),
                    asks,
                    bids,
                });

                ParseResult::Market(msg)
            }

            // --------------------------------------------------
            // Everything else:
            // subscriptions, heartbeat, errors, etc.
            // --------------------------------------------------
            _ => ParseResult::Control,
        }
    }
}
