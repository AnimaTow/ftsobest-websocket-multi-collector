use tokio_tungstenite::{connect_async, tungstenite::Message, tungstenite::Utf8Bytes};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use std::io::Read;
use tokio::sync::OnceCell;

use crate::{
    exchanges::adapter::{ExchangeAdapter, ChannelType},
    master_sender::MasterPool,
    config::ExchangeConfig,
};

static KUCOIN_WS_URL: OnceCell<String> = OnceCell::const_new();

async fn get_kucoin_ws_url() -> anyhow::Result<String> {
    KUCOIN_WS_URL
        .get_or_try_init(|| async {
            let res: serde_json::Value = reqwest::Client::new()
                .post("https://api.kucoin.com/api/v1/bullet-public")
                .send()
                .await?
                .json()
                .await?;

            let token = res["data"]["token"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("KuCoin token missing"))?;

            let endpoint = res["data"]["instanceServers"][0]["endpoint"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("KuCoin endpoint missing"))?;

            Ok(format!("{endpoint}?token={token}"))
        })
        .await
        .map(|s| s.clone())
}

/// Starts all collectors for a single exchange.
///
/// This function is responsible for:
/// - Spawning WebSocket connections for each enabled channel
/// - Applying channel-specific chunking strategies
/// - Keeping collectors alive indefinitely
///
/// DESIGN:
/// - One exchange → multiple WebSocket connections
/// - One channel may spawn multiple WS connections
/// - Failures are isolated per connection
///
/// This function does NOT:
/// - Perform reconnection logic (handled inside WS loop)
/// - Parse messages (delegated to adapters)
/// - Apply exchange-specific behavior
///
pub async fn run_exchange(
    adapter: Arc<dyn ExchangeAdapter>,
    cfg: ExchangeConfig,
    master: MasterPool,
) -> anyhow::Result<()> {

    // Spawn trade collectors (chunked)
    spawn_channel_chunks(
        adapter.clone(),
        cfg.clone(),
        ChannelType::Trades,
        master.clone(),
    );

    // Spawn orderbook collectors (one WS per pair)
    spawn_channel_chunks(
        adapter,
        cfg,
        ChannelType::OrderBooks,
        master,
    );

    Ok(())
}

/// Spawns WebSocket collectors for a specific channel.
///
/// Behavior depends on channel type:
/// - Trades: symbols are chunked per connection
/// - Orderbooks: exactly one WebSocket per symbol
///
/// RATIONALE:
/// - Trade channels allow batching to reduce WS connections
/// - Orderbook channels must be isolated to guarantee correctness
///
/// SAFETY:
/// - Each spawned task is fully isolated
/// - A failing WS does not affect others
///
fn spawn_channel_chunks(
    adapter: Arc<dyn ExchangeAdapter>,
    cfg: ExchangeConfig,
    channel: ChannelType,
    master: MasterPool,
) {
    match channel {

        // --------------------------------------------------
        // TRADES
        // --------------------------------------------------
        // Multiple symbols per WebSocket connection
        // Chunk size controlled via configuration
        //
        ChannelType::Trades => {
            let pairs = cfg.pairs.trades.clone();
            let chunk_size = cfg.chunking.trades_per_connection;

            for chunk in pairs.chunks(chunk_size) {
                let adapter = adapter.clone();
                let master = master.clone();
                let cfg = cfg.clone();
                let chunk_pairs = chunk.to_vec();

                tokio::spawn(async move {
                    run_ws_loop(
                        adapter,
                        cfg,
                        ChannelType::Trades,
                        chunk_pairs,
                        master,
                    ).await;
                });
            }
        }

        // --------------------------------------------------
        // ORDERBOOKS
        // --------------------------------------------------
        // Exactly one WebSocket connection per symbol
        //
        // IMPORTANT:
        // - Orderbook streams must not be mixed
        // - Mixing would break depth consistency and dedup logic
        //
        ChannelType::OrderBooks => {
            for pair in cfg.pairs.orderbooks.iter().cloned() {

                // Debug visibility for operational insight
                eprintln!(
                    "[ORDERBOOK] spawning WS for {} on {}",
                    pair,
                    adapter.name()
                );

                let adapter = adapter.clone();
                let master = master.clone();
                let cfg = cfg.clone();

                tokio::spawn(async move {
                    run_ws_loop(
                        adapter,
                        cfg,
                        ChannelType::OrderBooks,
                        vec![pair],
                        master,
                    ).await;
                });
            }
        }
    }
}

/// Runs a persistent WebSocket connection for exactly one symbol set.
///
/// This loop:
/// - Connects to the exchange WebSocket endpoint
/// - Subscribes to the requested channel
/// - Continuously reads messages
/// - Reconnects automatically on failure
///
/// GUARANTEES:
/// - This loop never exits voluntarily
/// - Failures cause a reconnect after a delay
///
/// RESPONSIBILITIES:
/// - Connection lifecycle
/// - Subscription sending
/// - Message forwarding to MasterPool
///
/// NOT RESPONSIBLE FOR:
/// - Message parsing (adapter responsibility)
/// - Data validation
/// - Deduplication (handled downstream)
///
/// TODO:
/// - Add exponential backoff
/// - Add connection-level metrics
/// - Add graceful shutdown support
///
async fn run_ws_loop(
    adapter: Arc<dyn ExchangeAdapter>,
    cfg: ExchangeConfig,
    channel: ChannelType,
    pairs: Vec<String>,
    master: MasterPool,
) {
    loop {
        let ws_url = if adapter.name() == "kucoin" {
            match get_kucoin_ws_url().await {
                Ok(url) => url,
                Err(e) => {
                    eprintln!("[KUCOIN] failed to fetch WS url: {e}");
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
            }
        } else {
            adapter.ws_url().to_string()
        };

        match connect_async(&ws_url).await {
            Ok((ws, _)) => {
                let (mut write, mut read) = ws.split();

                // Subscribe
                let sub = adapter.build_subscribe_message(channel, &pairs, &cfg);
                if write
                    .send(Message::Text(Utf8Bytes::from(sub.to_string())))
                    .await
                    .is_err()
                {
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }

                // READ LOOP
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Some(mm) = adapter.parse_message(&text, adapter.name()) {
                                let _ = master
                                    .send(serde_json::to_value(mm).unwrap())
                                    .await;
                            }
                        }

                        Ok(Message::Binary(bin)) => {
                            let mut decoder = flate2::read::GzDecoder::new(&bin[..]);
                            let mut decoded = String::new();

                            if decoder.read_to_string(&mut decoded).is_ok() {
                                if let Some(mm) = adapter.parse_message(&decoded, adapter.name()) {
                                    let _ = master
                                        .send(serde_json::to_value(mm).unwrap())
                                        .await;
                                }
                            }
                        }

                        Ok(Message::Ping(p)) => {
                            let _ = write.send(Message::Pong(p)).await;
                        }

                        Ok(Message::Close(_)) => break,

                        // ✅ CATCH-ALL für Pong, Frame, zukünftige Varianten
                        Ok(_) => {}

                        Err(_) => break,
                    }
                }
            }

            Err(e) => {
                eprintln!(
                    "WS connect failed [{} {:?}] – retry in 5s",
                    adapter.name(),
                    channel
                );
                eprintln!("   {}", e);
            }
        }

        sleep(Duration::from_secs(5)).await;
    }
}
