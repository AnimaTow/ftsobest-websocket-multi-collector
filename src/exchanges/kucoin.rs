use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType};

/// KuCoin WebSocket adapter
///
/// WICHTIG:
/// - Holt KEINE WS-URL
/// - Macht KEIN IO
/// - Übersetzt NUR Protokoll → MarketMessage
///
/// Trades only.
pub struct KucoinAdapter;

#[async_trait::async_trait]
impl ExchangeAdapter for KucoinAdapter {

    fn name(&self) -> &'static str {
        "kucoin"
    }

    /// Für KuCoin NICHT verwendet
    ///
    /// Die echte WS-URL wird vom Runner gesetzt
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
    ) -> Option<MarketMessage> {
        let v: Value = serde_json::from_str(raw).ok()?;

        // Nur echte Daten
        if v.get("type")?.as_str()? != "message" {
            return None;
        }

        let topic = v.get("topic")?.as_str()?;
        if !topic.starts_with("/market/match:") {
            return None;
        }

        let sym = topic.split(':').nth(1)?;
        let d = v.get("data")?;

        Some(MarketMessage::Trade(TradeData {
            exchange: exchange.to_string(),
            symbol: util::symbol_from_exchange(exchange, sym),
            timestamp: d["time"]
                .as_str()
                .and_then(|s| s.parse::<i128>().ok())
                .map(|ns| (ns / 1_000_000) as i64)
                .unwrap_or_else(util::now_ms),
            price: d["price"].as_str()?.to_string(),
            amount: d["size"].as_str()?.to_string(),
            side: d["side"].as_str()?.to_string(),
        }))
    }
}
