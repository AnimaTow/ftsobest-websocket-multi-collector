use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType, ParseResult};

/// KuCoin WebSocket adapter
///
/// IMPORTANT:
/// - Does NOT fetch WS URL
/// - Does NOT perform IO
/// - Pure protocol → MarketMessage translation
///
/// Trades only.
pub struct KucoinAdapter;

#[async_trait::async_trait]
impl ExchangeAdapter for KucoinAdapter {

    fn name(&self) -> &'static str {
        "kucoin"
    }

    /// Not used for KuCoin
    ///
    /// The real WS URL is injected by the runner
    fn ws_url(&self) -> &'static str {
        ""
    }

    fn build_subscribe_message(
        &self,
        channel: ChannelType,
        pairs: &[String],
        _config: &ExchangeConfig,
    ) -> Value {
        match channel {
            ChannelType::Trades => {
                let sym = util::symbol_to_exchange("kucoin", &pairs[0])
                    .to_uppercase(); // BTC-USDT

                json!({
                    "id": util::now_ms().to_string(),
                    "type": "subscribe",
                    "topic": format!("/market/match:{sym}"),
                    "privateChannel": false,
                    "response": true
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

        let msg_type = match v.get("type").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return ParseResult::Control,
        };

        // --------------------------------------------------
        // Control messages
        // --------------------------------------------------
        if msg_type != "message" {
            return ParseResult::Control;
        }

        let topic = match v.get("topic").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return ParseResult::Control,
        };

        if !topic.starts_with("/market/match:") {
            return ParseResult::Control;
        }

        let sym = match topic.split(':').nth(1) {
            Some(s) => s,
            None => return ParseResult::Error,
        };

        let d = match v.get("data") {
            Some(d) => d,
            None => return ParseResult::Error,
        };

        let timestamp = d.get("time")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<i128>().ok())
            .map(|ns| (ns / 1_000_000) as i64)
            .unwrap_or_else(util::now_ms);

        let msg = MarketMessage::Trade(TradeData {
            exchange: exchange.to_string(),
            symbol: util::symbol_from_exchange(exchange, sym),
            timestamp,
            price: d.get("price")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .to_string(),
            amount: d.get("size")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .to_string(),
            side: d.get("side")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
        });

        ParseResult::Market(msg)
    }
}
