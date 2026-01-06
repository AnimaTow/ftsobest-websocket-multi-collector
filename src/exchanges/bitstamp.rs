use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType, ParseResult};

/// Bitstamp WebSocket adapter (Spot trades)
///
/// WS:
/// wss://ws.bitstamp.net
///
/// Notes:
/// - No auth
/// - One trade per message
/// - price / amount are strings (perfect)
/// - type: 0 = buy, 1 = sell
pub struct BitstampAdapter;

#[async_trait::async_trait]
impl ExchangeAdapter for BitstampAdapter {

    fn name(&self) -> &'static str {
        "bitstamp"
    }

    fn ws_url(&self) -> &'static str {
        "wss://ws.bitstamp.net"
    }

    fn build_subscribe_message(
        &self,
        channel: ChannelType,
        pairs: &[String],
        _config: &ExchangeConfig,
    ) -> Value {

        match channel {
            ChannelType::Trades => {
                let pair = &pairs[0];

                let sym = util::symbol_to_exchange(self.name(), pair)
                    .to_lowercase();

                json!({
                "event": "bts:subscribe",
                "data": {
                    "channel": format!("live_trades_{}", sym)
                }
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
        println!("[RAW {}] {}", exchange, raw);
        let v: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return ParseResult::Error,
        };

        // Ignore control / non-trade messages
        if v.get("event").and_then(|v| v.as_str()) != Some("trade") {
            return ParseResult::Control;
        }

        let data = match v.get("data") {
            Some(d) => d,
            None => return ParseResult::Control,
        };

        let channel = v.get("channel")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // channel = live_trades_btcusd → btcusd
        let symbol_raw = channel
            .strip_prefix("live_trades_")
            .unwrap_or("");

        let symbol = util::symbol_from_exchange(exchange, symbol_raw);

        let price = data.get("price_str")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .to_string();

        let amount = data.get("amount_str")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .to_string();

        let timestamp = data.get("microtimestamp")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<i64>().ok())
            .map(|t| t / 1000) // µs → ms
            .unwrap_or_else(util::now_ms);

        let side = match data.get("type").and_then(|v| v.as_i64()) {
            Some(0) => "buy",
            Some(1) => "sell",
            _ => "unknown",
        }.to_string();

        let msg = MarketMessage::Trade(TradeData {
            exchange: exchange.to_string(),
            symbol,
            timestamp,
            price,
            amount,
            side,
        });

        ParseResult::Market(msg)
    }
}
