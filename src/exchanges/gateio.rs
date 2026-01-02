use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData, BookData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType, ParseResult};

/// Gate.io WebSocket adapter
pub struct GateIoAdapter;

#[async_trait::async_trait]
impl ExchangeAdapter for GateIoAdapter {

    fn name(&self) -> &'static str {
        "gateio"
    }

    fn ws_url(&self) -> &'static str {
        "wss://api.gateio.ws/ws/v4/"
    }

    fn build_subscribe_message(
        &self,
        channel: ChannelType,
        pairs: &[String],
        config: &ExchangeConfig,
    ) -> Value {

        let pairs: Vec<String> = pairs
            .iter()
            .map(|p| util::symbol_to_exchange(self.name(), p))
            .collect();

        match channel {
            ChannelType::Trades => json!({
                "time": util::now_ms(),
                "channel": "spot.trades",
                "event": "subscribe",
                "payload": pairs
            }),

            ChannelType::OrderBooks => {
                let depth = config
                    .orderbook
                    .as_ref()
                    .map(|o| o.depth)
                    .unwrap_or(20);

                let interval = config
                    .orderbook
                    .as_ref()
                    .map(|o| format!("{}ms", o.update_interval_ms))
                    .unwrap_or_else(|| "1000ms".to_string());

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

    fn parse_message(
        &self,
        raw: &str,
        exchange: &str,
    ) -> ParseResult {

        let v: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return ParseResult::Error,
        };

        let channel = match v.get("channel").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ParseResult::Control,
        };

        let event = match v.get("event").and_then(|v| v.as_str()) {
            Some(e) => e,
            None => return ParseResult::Control,
        };

        // --------------------------------------------------
        // Control messages
        // --------------------------------------------------
        if event != "update" {
            if event == "error" {
                return ParseResult::Error;
            }
            return ParseResult::Control;
        }

        // --------------------------------------------------
        // TRADES
        // --------------------------------------------------
        if channel == "spot.trades" {
            let r = &v["result"];

            let msg = MarketMessage::Trade(TradeData {
                exchange: exchange.to_string(),
                symbol: util::symbol_from_exchange(
                    exchange,
                    r.get("currency_pair")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                ),
                timestamp: r.get("create_time_ms")
                    .and_then(|v| v.as_i64())
                    .unwrap_or_else(util::now_ms),
                price: r.get("price")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0")
                    .to_string(),
                amount: r.get("amount")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0")
                    .to_string(),
                side: r.get("side")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
            });

            return ParseResult::Market(msg);
        }

        // --------------------------------------------------
        // ORDER BOOK
        // --------------------------------------------------
        if channel == "spot.order_book" {
            let r = &v["result"];

            let asks = r.get("asks")
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

            let bids = r.get("bids")
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
                symbol: util::symbol_from_exchange(
                    exchange,
                    r.get("s")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                ),
                timestamp: r.get("t")
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
