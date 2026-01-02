use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType};

/// Bitrue WebSocket adapter
///
/// ASSUMPTIONS:
/// - Exactly ONE pair per WebSocket connection
/// - WS client handles gzip (Binary frames)
/// - Trades only
pub struct BitrueAdapter;

#[async_trait::async_trait]
impl ExchangeAdapter for BitrueAdapter {

    fn name(&self) -> &'static str {
        "bitrue"
    }

    fn ws_url(&self) -> &'static str {
        "wss://fmarket-ws.bitrue.com/kline-api/ws"
    }

    fn build_subscribe_message(
        &self,
        channel: ChannelType,
        pairs: &[String],
        _config: &ExchangeConfig,
    ) -> Value {

        if pairs.is_empty() {
            return json!({});
        }

        match channel {
            ChannelType::Trades => {
                // Bitrue: exactly ONE symbol per WS
                let pair = &pairs[0];

                // BTC/USDT → btcusdt
                let sym = util::symbol_to_exchange(self.name(), pair)
                    .to_lowercase();

                json!({
                    "event": "sub",
                    "params": {
                        "channel": format!("market_e_{}_trade_ticker", sym)
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

        let channel = v.get("channel")?.as_str()?;
        if !channel.ends_with("trade_ticker") {
            return None;
        }

        // market_e_btcusdt_trade_ticker
        let sym = channel.split('_').nth(2)?;
        let symbol = util::symbol_from_exchange(exchange, sym);

        let trades = v
            .get("tick")?
            .get("data")?
            .as_array()?;

        let t = trades.first()?;

        Some(MarketMessage::Trade(TradeData {
            exchange: exchange.to_string(),
            symbol,
            timestamp: t.get("ts")
                .and_then(|v| v.as_i64())
                .unwrap_or_else(util::now_ms),
            price: t.get("price")?.as_str()?.to_string(),
            amount: t.get("amount")?.as_str()?.to_string(),
            side: t.get("side")?.as_str()?.to_lowercase(),
        }))
    }
}
