use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData, BookData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType, ParseResult};

/// Binance (Global) WebSocket adapter
///
/// Binance Spot WS:
/// https://developers.binance.com/docs/binance-spot-api-docs/websocket-market-streams
///
/// Supports MULTI combined streams per connection.
pub struct BinanceAdapter;

#[async_trait::async_trait]
impl ExchangeAdapter for BinanceAdapter {

    fn name(&self) -> &'static str {
        "binance"
    }

    fn ws_url(&self) -> &'static str {
        "wss://stream.binance.com:9443/ws"
    }

    fn build_subscribe_message(
        &self,
        channel: ChannelType,
        pairs: &[String],
        _config: &ExchangeConfig,
    ) -> Value {

        let streams: Vec<String> = pairs.iter().map(|p| {
            let symbol = util::symbol_to_exchange(self.name(), p).to_lowercase();

            match channel {
                ChannelType::Trades => {
                    format!("{}@trade", symbol)
                }

                ChannelType::OrderBooks => {
                    // Binance Global supports depth params,
                    // but we intentionally use the safest default
                    format!("{}@depth", symbol)
                }
            }
        }).collect();

        json!({
            "method": "SUBSCRIBE",
            "params": streams,
            "id": util::now_ms()
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
        // Binance control / ack messages
        // --------------------------------------------------
        // Example:
        // { "result": null, "id": 123 }
        if v.get("result").is_some() {
            return ParseResult::Control;
        }

        // --------------------------------------------------
        // Binance Combined Stream Wrapper
        // --------------------------------------------------
        let data = if let Some(d) = v.get("data") {
            d
        } else {
            &v
        };

        let event = match data.get("e").and_then(|e| e.as_str()) {
            Some(e) => e,
            None => return ParseResult::Control, // ping / keepalive / unknown control
        };

        match event {

            // -----------------------------
            // TRADES
            // -----------------------------
            "trade" => {
                let msg = MarketMessage::Trade(TradeData {
                    exchange: exchange.to_string(),
                    symbol: util::symbol_from_exchange(
                        exchange,
                        data["s"].as_str().unwrap_or_default()
                    ),
                    timestamp: data["T"]
                        .as_i64()
                        .unwrap_or_else(util::now_ms),
                    price: data["p"].as_str().unwrap_or("0").to_string(),
                    amount: data["q"].as_str().unwrap_or("0").to_string(),
                    side: if data["m"].as_bool().unwrap_or(false) {
                        "sell".into()
                    } else {
                        "buy".into()
                    },
                });

                ParseResult::Market(msg)
            }

            // -----------------------------
            // ORDER BOOK (delta)
            // -----------------------------
            "depthUpdate" => {
                let asks = data["a"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|x| {
                        let price = x.get(0)?.as_str()?;
                        let qty   = x.get(1)?.as_str()?;
                        if qty == "0.00000000" {
                            return None;
                        }
                        Some([price.to_string(), qty.to_string()])
                    })
                    .collect();

                let bids = data["b"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|x| {
                        let price = x.get(0)?.as_str()?;
                        let qty   = x.get(1)?.as_str()?;
                        if qty == "0.00000000" {
                            return None;
                        }
                        Some([price.to_string(), qty.to_string()])
                    })
                    .collect();

                let msg = MarketMessage::Book(BookData {
                    exchange: exchange.to_string(),
                    symbol: util::symbol_from_exchange(
                        exchange,
                        data["s"].as_str().unwrap_or_default()
                    ),
                    timestamp: data["E"]
                        .as_i64()
                        .unwrap_or_else(util::now_ms),
                    asks,
                    bids,
                });

                ParseResult::Market(msg)
            }

            // -----------------------------
            // Everything else
            // -----------------------------
            _ => ParseResult::Control,
        }
    }

}
