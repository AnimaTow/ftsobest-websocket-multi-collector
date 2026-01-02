use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType, ParseResult};

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
    ) -> ParseResult {

        let v: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return ParseResult::Error,
        };

        // --------------------------------------------------
        // Control / heartbeat / non-trade messages
        // --------------------------------------------------
        let channel = match v.get("channel").and_then(|c| c.as_str()) {
            Some(c) => c,
            None => return ParseResult::Control,
        };

        if !channel.ends_with("trade_ticker") {
            return ParseResult::Control;
        }

        // --------------------------------------------------
        // Extract symbol
        // market_e_btcusdt_trade_ticker
        // --------------------------------------------------
        let sym = match channel.split('_').nth(2) {
            Some(s) => s,
            None => return ParseResult::Error,
        };

        let symbol = util::symbol_from_exchange(exchange, sym);

        // --------------------------------------------------
        // Extract trades
        // --------------------------------------------------
        let trades = match v
            .get("tick")
            .and_then(|t| t.get("data"))
            .and_then(|d| d.as_array())
        {
            Some(t) if !t.is_empty() => t,
            _ => return ParseResult::Control,
        };

        let t = &trades[0];

        let msg = MarketMessage::Trade(TradeData {
            exchange: exchange.to_string(),
            symbol,
            timestamp: t.get("ts")
                .and_then(|v| v.as_i64())
                .unwrap_or_else(util::now_ms),
            price: t.get("price")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .to_string(),
            amount: t.get("amount")
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
