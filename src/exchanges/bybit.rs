use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData, BookData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType, ParseResult};

/// Bybit Spot WebSocket adapter
///
/// WS:
/// wss://stream.bybit.com/v5/public/spot
///
/// Channels:
/// - publicTrade.{symbol}
/// - orderbook.50.{symbol}
pub struct BybitAdapter;

#[async_trait::async_trait]
impl ExchangeAdapter for BybitAdapter {

    fn name(&self) -> &'static str {
        "bybit"
    }

    fn ws_url(&self) -> &'static str {
        "wss://stream.bybit.com/v5/public/spot"
    }

    fn build_subscribe_message(
        &self,
        channel: ChannelType,
        pairs: &[String],
        _config: &ExchangeConfig,
    ) -> Value {

        let topics: Vec<String> = pairs.iter().map(|p| {
            let symbol = util::symbol_to_exchange(self.name(), p); // BTCUSDT

            match channel {
                ChannelType::Trades =>
                    format!("publicTrade.{}", symbol),

                ChannelType::OrderBooks =>
                    format!("orderbook.50.{}", symbol),
            }
        }).collect();

        json!({
            "op": "subscribe",
            "args": topics
        })
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

        // --------------------------------------------------
        // Control messages (subscribe ack, pong, etc.)
        // --------------------------------------------------
        if v.get("op").is_some() {
            return ParseResult::Control;
        }

        let topic = match v.get("topic").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => return ParseResult::Control,
        };

        let data = match v.get("data") {
            Some(d) => d,
            None => return ParseResult::Control,
        };

        // --------------------------------------------------
        // TRADES
        // --------------------------------------------------
        if topic.starts_with("publicTrade.") {

            let trades = match data.as_array() {
                Some(t) if !t.is_empty() => t,
                _ => return ParseResult::Control,
            };

            let t = &trades[0];

            let msg = MarketMessage::Trade(TradeData {
                exchange: exchange.to_string(),
                symbol: util::symbol_from_exchange(
                    exchange,
                    t.get("s").and_then(|v| v.as_str()).unwrap_or_default()
                ),
                timestamp: t.get("T")
                    .and_then(|v| v.as_i64())
                    .unwrap_or_else(util::now_ms),
                price: t.get("p")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0")
                    .to_string(),
                amount: t.get("v")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0")
                    .to_string(),
                side: t.get("S")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_lowercase(),
            });

            return ParseResult::Market(msg);
        }

        // --------------------------------------------------
        // ORDER BOOK
        // --------------------------------------------------
        if topic.starts_with("orderbook.") {

            let symbol = match data.get("s").and_then(|v| v.as_str()) {
                Some(s) => s,
                None => return ParseResult::Error,
            };

            let asks = data.get("a")
                .and_then(|v| v.as_array())
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|x| {
                    Some([
                        x.get(0)?.as_str()?.to_string(),
                        x.get(1)?.as_str()?.to_string(),
                    ])
                })
                .collect::<Vec<[String; 2]>>();

            let bids = data.get("b")
                .and_then(|v| v.as_array())
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|x| {
                    Some([
                        x.get(0)?.as_str()?.to_string(),
                        x.get(1)?.as_str()?.to_string(),
                    ])
                })
                .collect::<Vec<[String; 2]>>();

            let msg = MarketMessage::Book(BookData {
                exchange: exchange.to_string(),
                symbol: util::symbol_from_exchange(exchange, symbol),
                timestamp: data.get("ts")
                    .and_then(|v| v.as_i64())
                    .unwrap_or_else(util::now_ms),
                asks,
                bids,
            });

            return ParseResult::Market(msg);
        }

        ParseResult::Control
    }
}
