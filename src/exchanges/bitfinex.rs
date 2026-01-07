use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use crate::{
    util,
    schema::{MarketMessage, TradeData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType, ParseResult};

/// Bitfinex WebSocket adapter (Spot trades)
///
/// WS:
/// wss://api-pub.bitfinex.com/ws/2
///
/// Supports:
/// - Multiple symbols per WS
/// - Channel-ID routing
/// - Trade batches via internal buffer
pub struct BitfinexAdapter {
    /// chanId → symbol
    chan_map: Mutex<HashMap<i64, String>>,

    /// Parsed trades waiting to be emitted
    trade_buffer: Mutex<VecDeque<MarketMessage>>,
}

impl BitfinexAdapter {
    pub fn new() -> Self {
        Self {
            chan_map: Mutex::new(HashMap::new()),
            trade_buffer: Mutex::new(VecDeque::with_capacity(64)),
        }
    }
}

/// Safe numeric → string (NO scientific notation)
fn num_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => "0".to_string(),
    }
}

fn normalize_amount_decimal(v: &Value) -> String {
    let f = v.as_f64().unwrap_or(0.0).abs();

    let s = format!("{:.12}", f);

    s.trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[async_trait::async_trait]
impl ExchangeAdapter for BitfinexAdapter {

    fn name(&self) -> &'static str {
        "bitfinex"
    }

    fn ws_url(&self) -> &'static str {
        "wss://api-pub.bitfinex.com/ws/2"
    }

    fn build_subscribe_message(
        &self,
        channel: ChannelType,
        pairs: &[String],
        _config: &ExchangeConfig,
    ) -> Value {
        match channel {
            ChannelType::Trades => {
                // Bitfinex: ONLY FIRST SYMBOL per message
                json!({
                "event": "subscribe",
                "channel": "trades",
                "symbol": util::symbol_to_exchange(self.name(), &pairs[0])
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
        //println!("[RAW {}] {}", exchange, raw);
        // 1️⃣ Emit buffered trades first
        if let Some(msg) = self.trade_buffer.lock().unwrap().pop_front() {
            return ParseResult::Market(msg);
        }

        let v: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return ParseResult::Error,
        };

        // --------------------------------------------------
        // Control / subscribe messages (objects)
        // --------------------------------------------------
        if v.is_object() {
            if v.get("event").and_then(|v| v.as_str()) == Some("subscribed") {
                if v.get("channel").and_then(|v| v.as_str()) == Some("trades") {
                    if let (Some(chan_id), Some(symbol)) = (
                        v.get("chanId").and_then(|v| v.as_i64()),
                        v.get("symbol").and_then(|v| v.as_str()),
                    ) {
                        let norm = util::symbol_from_exchange(exchange, symbol);
                        self.chan_map.lock().unwrap().insert(chan_id, norm);
                    }
                }
            }
            return ParseResult::Control;
        }

        // --------------------------------------------------
        // Trade frames (arrays)
        // --------------------------------------------------
        let arr = match v.as_array() {
            Some(a) if a.len() >= 2 => a,
            _ => return ParseResult::Control,
        };

        let chan_id = match arr.get(0).and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return ParseResult::Control,
        };

        let msg_type = arr.get(1).and_then(|v| v.as_str());

        // Ignore snapshots & heartbeats
        if msg_type != Some("tu") {
            return ParseResult::Control;
        }

        let trade = match arr.get(2).and_then(|v| v.as_array()) {
            Some(t) if t.len() >= 4 => t,
            _ => return ParseResult::Control,
        };

        let symbol = match self.chan_map.lock().unwrap().get(&chan_id) {
            Some(s) => s.clone(),
            None => return ParseResult::Control,
        };

        let ts = trade.get(1)
            .and_then(|v| v.as_i64())
            .unwrap_or_else(util::now_ms);

        let amount_val = trade.get(2).unwrap_or(&Value::Null);
        let price_val  = trade.get(3).unwrap_or(&Value::Null);

        let price      = num_to_string(price_val);

        let amount_f = amount_val.as_f64().unwrap_or(0.0);

        let side = if amount_f > 0.0 {
            "buy"
        } else {
            "sell"
        }.to_string();

        let amount = normalize_amount_decimal(amount_val);

        let msg = MarketMessage::Trade(TradeData {
            exchange: exchange.to_string(),
            symbol,
            timestamp: ts,
            price,
            amount,
            side,
        });

        self.trade_buffer.lock().unwrap().push_back(msg);

        self.trade_buffer
            .lock()
            .unwrap()
            .pop_front()
            .map(ParseResult::Market)
            .unwrap_or(ParseResult::Control)
    }
}
