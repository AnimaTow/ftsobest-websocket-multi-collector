use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData, BookData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType};

/// Coinbase WebSocket adapter
///
/// Coinbase Advanced Trade WS:
/// wss://advanced-trade-ws.coinbase.com
///
/// Channels:
/// - matches  → trades
/// - level2   → order book deltas
///
/// DESIGN:
/// - Pure protocol translation
/// - No reconnect logic
/// - No storage / master logic
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

        // BTC/USD → BTC-USD
        let product_ids: Vec<String> = pairs
            .iter()
            .map(|p| util::symbol_to_exchange(self.name(), p))
            .collect();

        match channel {

            // -----------------------------
            // TRADES
            // -----------------------------
            ChannelType::Trades => json!({
                "type": "subscribe",
                "product_ids": product_ids,
                "channels": ["matches"]
            }),

            // -----------------------------
            // ORDER BOOK (L2 deltas)
            // -----------------------------
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
    ) -> Option<MarketMessage> {

        let v: Value = serde_json::from_str(raw).ok()?;
        let msg_type = v.get("type")?.as_str()?;

        match msg_type {

            // -----------------------------
            // TRADES
            // -----------------------------
            "match" => Some(MarketMessage::Trade(TradeData {
                exchange: exchange.to_string(),
                symbol: util::symbol_from_exchange(
                    exchange,
                    v.get("product_id")?.as_str()?
                ),
                timestamp: util::now_ms(),
                price: v.get("price")?.as_str()?.to_string(),
                amount: v.get("size")?.as_str()?.to_string(),
                side: v.get("side")?.as_str()?.to_string(),
            })),

            // -----------------------------
            // ORDER BOOK (delta)
            // -----------------------------
            "l2update" => {
                let product_id = v.get("product_id")?.as_str()?;
                let changes = v.get("changes")?.as_array()?;

                let mut bids = Vec::new();
                let mut asks = Vec::new();

                for c in changes {
                    let side = c.get(0)?.as_str()?;
                    let price = c.get(1)?.as_str()?.to_string();
                    let qty   = c.get(2)?.as_str()?.to_string();

                    if qty == "0" {
                        continue;
                    }

                    match side {
                        "buy"  => bids.push([price, qty]),
                        "sell" => asks.push([price, qty]),
                        _ => {}
                    }
                }

                Some(MarketMessage::Book(BookData {
                    exchange: exchange.to_string(),
                    symbol: util::symbol_from_exchange(exchange, product_id),
                    timestamp: util::now_ms(),
                    asks,
                    bids,
                }))
            }

            _ => None,
        }
    }
}
