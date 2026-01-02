use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType, ParseResult};

/// OKX WebSocket adapter
///
/// OKX Spot WS v5:
/// https://www.okx.com/docs-v5/en/#websocket-api-public-channel-trades
///
/// DESIGN:
/// - Pure protocol translation
/// - No reconnect logic
/// - No chunking
/// - No state
pub struct OkxAdapter;

#[async_trait::async_trait]
impl ExchangeAdapter for OkxAdapter {

    fn name(&self) -> &'static str {
        "okx"
    }

    fn ws_url(&self) -> &'static str {
        "wss://ws.okx.com:8443/ws/v5/public"
    }

    fn build_subscribe_message(
        &self,
        channel: ChannelType,
        pairs: &[String],
        _config: &ExchangeConfig,
    ) -> Value {

        match channel {
            ChannelType::Trades => {
                let args: Vec<Value> = pairs.iter().map(|p| {
                    let inst_id = util::symbol_to_exchange(self.name(), p);
                    json!({
                        "channel": "trades",
                        "instId": inst_id
                    })
                }).collect();

                json!({
                    "op": "subscribe",
                    "args": args
                })
            }

            ChannelType::OrderBooks => json!({}),
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

        // --------------------------------------------------
        // Control / error messages
        // --------------------------------------------------
        if let Some(event) = v.get("event").and_then(|v| v.as_str()) {
            if event == "error" {
                return ParseResult::Error;
            }
            return ParseResult::Control; // subscribe, unsubscribe, etc.
        }

        let arg = match v.get("arg") {
            Some(a) => a,
            None => return ParseResult::Control,
        };

        let channel = match arg.get("channel").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ParseResult::Control,
        };

        if channel != "trades" {
            return ParseResult::Control;
        }

        let inst_id = match arg.get("instId").and_then(|v| v.as_str()) {
            Some(i) => i,
            None => return ParseResult::Error,
        };

        let symbol = util::symbol_from_exchange(exchange, inst_id);

        let trades = match v.get("data").and_then(|v| v.as_array()) {
            Some(t) if !t.is_empty() => t,
            _ => return ParseResult::Control,
        };

        let t = &trades[0];

        let msg = MarketMessage::Trade(TradeData {
            exchange: exchange.to_string(),
            symbol,
            timestamp: t.get("ts")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or_else(util::now_ms),
            price: t.get("px")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .to_string(),
            amount: t.get("sz")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .to_string(),
            side: t.get("side")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_lowercase(),
        });

        ParseResult::Market(msg)
    }
}
