use serde_json::{Value, json};

use crate::{
    util,
    schema::{MarketMessage, TradeData, BookData},
    config::ExchangeConfig,
};

use super::adapter::{ExchangeAdapter, ChannelType};

/// Binance US WebSocket adapter
///
/// Binance US Spot WS:
/// https://docs.binance.us/#websocket-market-streams
///
/// Supports MULTI stream subscriptions per connection.
pub struct BinanceUsAdapter;

#[async_trait::async_trait]
impl ExchangeAdapter for BinanceUsAdapter {

    fn name(&self) -> &'static str {
        "binanceus"
    }

    fn ws_url(&self) -> &'static str {
        "wss://stream.binance.us:9443/ws"
    }

    fn build_subscribe_message(
        &self,
        channel: ChannelType,
        pairs: &[String],
        config: &ExchangeConfig,
    ) -> Value {
        let _config = config;
        let streams: Vec<String> = pairs.iter().map(|p| {
            let symbol = util::symbol_to_exchange(self.name(), p).to_lowercase();

            match channel {
                ChannelType::Trades => {
                    format!("{}@trade", symbol)
                }

                ChannelType::OrderBooks => {
                    format!("{}@depth", symbol)
                }
            }
        }).collect();

        json!({
            "method": "SUBSCRIBE",
            "params": streams,
            "id": util::now_ms()
        })
    }

    fn parse_message(
        &self,
        raw: &str,
        exchange: &str,
    ) -> Option<MarketMessage> {

        let v: Value = serde_json::from_str(raw).ok()?;

        // Binance Combined Stream Wrapper
        let (_stream, data) = if v.get("stream").is_some() {
            (
                v.get("stream")?.as_str()?,
                v.get("data")?
            )
        } else {
            // Fallback: single stream format
            ("", &v)
        };

        let event = data.get("e")?.as_str()?;

        match event {

            "trade" => Some(MarketMessage::Trade(TradeData {
                exchange: exchange.to_string(),
                symbol: util::symbol_from_exchange(
                    exchange,
                    data["s"].as_str()?
                ),
                timestamp: data["T"].as_i64().unwrap_or_else(util::now_ms),
                price: data["p"].as_str()?.to_string(),
                amount: data["q"].as_str()?.to_string(),
                side: if data["m"].as_bool()? {
                    "sell".into()
                } else {
                    "buy".into()
                },
            })),

            "depthUpdate" => {
                let asks = data["a"].as_array()?.iter().filter_map(|x| {
                    let price = x.get(0)?.as_str()?;
                    let qty   = x.get(1)?.as_str()?;
                    if qty == "0.00000000" {
                        return None;
                    }
                    Some([price.to_string(), qty.to_string()])
                }).collect();

                let bids = data["b"].as_array()?.iter().filter_map(|x| {
                    let price = x.get(0)?.as_str()?;
                    let qty   = x.get(1)?.as_str()?;
                    if qty == "0.00000000" {
                        return None;
                    }
                    Some([price.to_string(), qty.to_string()])
                }).collect();

                Some(MarketMessage::Book(BookData {
                    exchange: exchange.to_string(),
                    symbol: util::symbol_from_exchange(
                        exchange,
                        data["s"].as_str()?
                    ),
                    timestamp: data["E"].as_i64().unwrap_or_else(util::now_ms),
                    asks,
                    bids,
                }))
            }

            _ => None,
        }
    }
}
