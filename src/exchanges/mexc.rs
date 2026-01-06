use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType, ParseResult};

/// MEXC WebSocket adapter (Futures deal stream)
///
/// WS:
/// wss://contract.mexc.com/edge
///
/// Notes:
/// - No auth
/// - No token
/// - Trades only
/// - One symbol per WS connection (recommended)
pub struct MexcAdapter;

#[async_trait::async_trait]
impl ExchangeAdapter for MexcAdapter {

    fn name(&self) -> &'static str {
        "mexc"
    }

    fn ws_url(&self) -> &'static str {
        "wss://contract.mexc.com/edge"
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

                // BTC/USDT -> BTC_USDT
                let symbol = util::symbol_to_exchange("mexc", pair)
                    .replace('-', "_")
                    .to_uppercase();

                json!({
                    "method": "sub.deal",
                    "param": {
                        "symbol": symbol
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

        let v: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return ParseResult::Error,
        };

        let channel = match v.get("channel").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ParseResult::Control,
        };

        // --------------------------------------------------
        // Only deal pushes are relevant
        // --------------------------------------------------
        if channel != "push.deal" {
            return ParseResult::Control;
        }

        let symbol_raw = match v.get("symbol").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return ParseResult::Error,
        };

        let symbol = util::symbol_from_exchange(exchange, symbol_raw);

        let trades = match v.get("data").and_then(|v| v.as_array()) {
            Some(t) if !t.is_empty() => t,
            _ => return ParseResult::Control,
        };

        let t = &trades[0];

        let side = match t.get("T").and_then(|v| v.as_i64()) {
            Some(1) => "buy",
            Some(2) => "sell",
            _ => "unknown",
        }.to_string();

        let msg = MarketMessage::Trade(TradeData {
            exchange: exchange.to_string(),
            symbol,
            timestamp: t.get("t")
                .and_then(|v| v.as_i64())
                .unwrap_or_else(util::now_ms),
            price: t.get("p")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "0".to_string()),
            amount: t.get("v")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "0".to_string()),
            side,
        });

        ParseResult::Market(msg)
    }
}
