use serde_json::{Value, json};
use std::collections::VecDeque;
use std::sync::Mutex;

use crate::{
    util,
    schema::{MarketMessage, TradeData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType, ParseResult};

/// Kraken WebSocket v2 adapter (Spot)
///
/// WS:
/// wss://ws.kraken.com/v2
///
/// Supports:
/// - Trade batches
/// - Multiple symbols per WS
/// - Future orderbook extension
pub struct KrakenV2Adapter {
    trade_buffer: Mutex<VecDeque<MarketMessage>>,
}

impl KrakenV2Adapter {
    pub fn new() -> Self {
        Self {
            trade_buffer: Mutex::new(VecDeque::with_capacity(32)),
        }
    }
}

/// Safe numeric extraction (NO floats, NO scientific notation)
fn val_to_string(v: Option<&Value>, max_decimals: usize) -> String {
    match v {
        // Kraken sendet manchmal echte Strings → perfekt
        Some(Value::String(s)) => s.clone(),

        // Numbers → kontrolliert formatieren
        Some(Value::Number(n)) => {
            if let Some(f) = n.as_f64() {
                let s = format!("{:.*}", max_decimals, f);
                s.trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            } else {
                "0".to_string()
            }
        }

        _ => "0".to_string(),
    }
}

#[async_trait::async_trait]
impl ExchangeAdapter for KrakenV2Adapter {

    fn name(&self) -> &'static str {
        "kraken"
    }

    fn ws_url(&self) -> &'static str {
        "wss://ws.kraken.com/v2"
    }

    fn build_subscribe_message(
        &self,
        channel: ChannelType,
        pairs: &[String],
        _config: &ExchangeConfig,
    ) -> Value {

        let symbols: Vec<String> = pairs
            .iter()
            .map(|p| util::symbol_to_exchange(self.name(), p))
            .collect();

        match channel {
            ChannelType::Trades => json!({
                "method": "subscribe",
                "params": {
                    "channel": "trade",
                    "symbol": symbols
                }
            }),

            // Prepared but not yet emitted
            ChannelType::OrderBooks => json!({
                "method": "subscribe",
                "params": {
                    "channel": "book",
                    "symbol": symbols,
                    "depth": 20
                }
            }),
        }
    }

    fn parse_message(
        &self,
        raw: &str,
        exchange: &str,
    ) -> ParseResult {

        // 1️⃣ Emit buffered trades first
        if let Some(msg) = self.trade_buffer.lock().unwrap().pop_front() {
            return ParseResult::Market(msg);
        }

        let v: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return ParseResult::Error,
        };

        // 2️⃣ Ignore heartbeats & control
        let channel = v.get("channel").and_then(|v| v.as_str());
        let msg_type = v.get("type").and_then(|v| v.as_str());

        if channel == Some("heartbeat") {
            return ParseResult::Control;
        }

        if msg_type != Some("update") {
            return ParseResult::Control;
        }

        // 3️⃣ Trades
        if channel == Some("trade") {
            let trades = match v.get("data").and_then(|v| v.as_array()) {
                Some(d) if !d.is_empty() => d,
                _ => return ParseResult::Control,
            };
            let mut buffer = self.trade_buffer.lock().unwrap();

            for t in trades {
                let symbol_raw = t.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
                let symbol = util::symbol_from_exchange(exchange, symbol_raw);

                let price  = val_to_string(t.get("price"), 10);
                let amount = val_to_string(t.get("qty"), 10);

                let ts = t.get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.timestamp_millis())
                    .unwrap_or_else(util::now_ms);

                let side = t.get("side")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                buffer.push_back(MarketMessage::Trade(TradeData {
                    exchange: exchange.to_string(),
                    symbol,
                    timestamp: ts,
                    price,
                    amount,
                    side,
                }));
            }

            return buffer
                .pop_front()
                .map(ParseResult::Market)
                .unwrap_or(ParseResult::Control);
        }

        // 4️⃣ Orderbook v2 placeholder
        if channel == Some("book") {
            // TODO:
            // - snapshot vs update
            // - asks / bids parsing
            // - BookData emit
            return ParseResult::Control;
        }

        ParseResult::Control
    }
}
