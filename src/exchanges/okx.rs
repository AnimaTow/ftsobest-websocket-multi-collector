use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType};

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

            // OKX order books intentionally not implemented here
            ChannelType::OrderBooks => json!({}),
        }
    }

    fn parse_message(
        &self,
        raw: &str,
        exchange: &str,
    ) -> Option<MarketMessage> {

        let v: Value = serde_json::from_str(raw).ok()?;

        let arg = v.get("arg")?;
        let channel = arg.get("channel")?.as_str()?;
        if channel != "trades" {
            return None;
        }

        let inst_id = arg.get("instId")?.as_str()?;
        let symbol = util::symbol_from_exchange(exchange, inst_id);

        let trades = v.get("data")?.as_array()?;
        let t = trades.first()?;

        Some(MarketMessage::Trade(TradeData {
            exchange: exchange.to_string(),
            symbol,
            timestamp: t.get("ts")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or_else(util::now_ms),
            price: t.get("px")?.as_str()?.to_string(),
            amount: t.get("sz")?.as_str()?.to_string(),
            side: t.get("side")?.as_str()?.to_lowercase(),
        }))
    }

}
