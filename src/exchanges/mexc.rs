use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType};

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
///
/// DESIGN:
/// - Pure protocol translation
/// - No reconnect logic
/// - No business logic
/// - Fully compatible with generic runner
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
                // MEXC strongly prefers one symbol per connection
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
    ) -> Option<MarketMessage> {

        let v: Value = serde_json::from_str(raw).ok()?;

        // We only care about deal pushes
        if v.get("channel")?.as_str()? != "push.deal" {
            return None;
        }

        let symbol_raw = v.get("symbol")?.as_str()?;
        let symbol = util::symbol_from_exchange(exchange, symbol_raw);

        let trades = v.get("data")?.as_array()?;
        let t = trades.first()?; // usually exactly one trade

        Some(MarketMessage::Trade(TradeData {
            exchange: exchange.to_string(),
            symbol,
            timestamp: t.get("t")
                .and_then(|v| v.as_i64())
                .unwrap_or_else(util::now_ms),
            price: t.get("p")?.to_string(),
            amount: t.get("v")?.to_string(),
            side: match t.get("S")?.as_i64()? {
                1 => "buy".into(),
                2 => "sell".into(),
                _ => "unknown".into(),
            },
        }))
    }
}
