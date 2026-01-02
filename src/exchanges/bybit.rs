use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData, BookData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType};

/// Bybit Spot WebSocket adapter
///
/// WS:
/// wss://stream.bybit.com/v5/public/spot
///
/// Channels:
/// - publicTrade.{symbol}
/// - orderbook.50.{symbol}
///
/// DESIGN:
/// - Pure protocol translation
/// - No reconnect logic
/// - No runtime logic
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
    ) -> Option<MarketMessage> {

        let v: Value = serde_json::from_str(raw).ok()?;

        let topic = v.get("topic")?.as_str()?;
        let data  = v.get("data")?;

        // --------------------------------------------------
        // TRADES
        // --------------------------------------------------
        if topic.starts_with("publicTrade.") {
            let trades = data.as_array()?;
            let t = trades.first()?;

            return Some(MarketMessage::Trade(TradeData {
                exchange: exchange.to_string(),
                symbol: util::symbol_from_exchange(
                    exchange,
                    t.get("s")?.as_str()?
                ),
                timestamp: t.get("T")
                    .and_then(|v| v.as_i64())
                    .unwrap_or_else(util::now_ms),
                price: t.get("p")?.as_str()?.to_string(),
                amount: t.get("v")?.as_str()?.to_string(),
                side: t.get("S")?.as_str()?.to_lowercase(),
            }));
        }

        // --------------------------------------------------
        // ORDER BOOK (delta)
        // --------------------------------------------------
        if topic.starts_with("orderbook.") {
            let symbol = data.get("s")?.as_str()?;

            let asks = data.get("a")?
                .as_array()?
                .iter()
                .filter_map(|x| {
                    Some([
                        x.get(0)?.as_str()?.to_string(),
                        x.get(1)?.as_str()?.to_string(),
                    ])
                })
                .collect::<Vec<[String; 2]>>();

            let bids = data.get("b")?
                .as_array()?
                .iter()
                .filter_map(|x| {
                    Some([
                        x.get(0)?.as_str()?.to_string(),
                        x.get(1)?.as_str()?.to_string(),
                    ])
                })
                .collect::<Vec<[String; 2]>>();

            return Some(MarketMessage::Book(BookData {
                exchange: exchange.to_string(),
                symbol: util::symbol_from_exchange(exchange, symbol),
                timestamp: data.get("ts")
                    .and_then(|v| v.as_i64())
                    .unwrap_or_else(util::now_ms),
                asks,
                bids,
            }));
        }

        None
    }
}
