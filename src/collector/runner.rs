use tokio_tungstenite::{connect_async, tungstenite::Message, tungstenite::Utf8Bytes};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use std::io::Read;
use tokio::sync::OnceCell;
use std::sync::atomic::Ordering;

use crate::metrics::METRICS;
use crate::{exchanges::adapter::{ExchangeAdapter, ChannelType, ParseResult}, master_sender::MasterPool, config::ExchangeConfig, util};

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

pub async fn run_exchange(
    adapter: Arc<dyn ExchangeAdapter>,
    cfg: ExchangeConfig,
    master: MasterPool,
) -> anyhow::Result<()> {
    spawn_channel_chunks(
        adapter.clone(),
        cfg.clone(),
        ChannelType::Trades,
        master.clone(),
    );

    spawn_channel_chunks(
        adapter,
        cfg,
        ChannelType::OrderBooks,
        master,
    );

    Ok(())
}

fn spawn_channel_chunks(
    adapter: Arc<dyn ExchangeAdapter>,
    cfg: ExchangeConfig,
    channel: ChannelType,
    master: MasterPool,
) {
    match channel {
        ChannelType::Trades => {
            let pairs = cfg.pairs.trades.clone();

            METRICS
                .trade_pairs_active
                .fetch_add(pairs.len(), Ordering::Relaxed);

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
                    )
                        .await;
                });
            }
        }

        ChannelType::OrderBooks => {
            let pairs = cfg.pairs.orderbooks.clone();

            METRICS
                .orderbook_pairs_active
                .fetch_add(pairs.len(), Ordering::Relaxed);

            for pair in pairs {
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
                    )
                        .await;
                });
            }
        }
    }
}

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
                METRICS
                    .ws_connections_active
                    .fetch_add(1, Ordering::Relaxed);

                let (write, mut read) = ws.split();
                let write = Arc::new(tokio::sync::Mutex::new(write));

                // ---- KUCOIN CLIENT PING LOOP ----
                if adapter.name() == "kucoin" {
                    let ping_interval = ws_url
                        .split("|ping=")
                        .nth(1)
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(20000);

                    let ping_write = write.clone();

                    let ping_every = Duration::from_millis(ping_interval / 2);

                    tokio::spawn(async move {
                        loop {
                            sleep(ping_every).await;

                            let ping = serde_json::json!({
                "type": "ping",
                "id": util::now_ms().to_string()
            });

                            if ping_write
                                .lock()
                                .await
                                .send(Message::Text(Utf8Bytes::from(ping.to_string())))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                    });
                }


                match adapter.name() {
                    // Exchanges that require ONE subscribe per symbol
                    "bitfinex" | "bitstamp" => {
                        for pair in &pairs {
                            let sub = adapter.build_subscribe_message(
                                channel,
                                &[pair.clone()],
                                &cfg,
                            );

                            if write
                                .lock()
                                .await
                                .send(Message::Text(Utf8Bytes::from(sub.to_string())))
                                .await
                                .is_err()
                            {
                                METRICS.subscription_errors.fetch_add(1, Ordering::Relaxed);
                                METRICS
                                    .ws_connections_active
                                    .fetch_sub(1, Ordering::Relaxed);
                                break;
                            }

                            METRICS.subscriptions_sent.fetch_add(1, Ordering::Relaxed);
                        }
                    }

                    // Exchanges that support batch subscribe
                    _ => {
                        let sub = adapter.build_subscribe_message(channel, &pairs, &cfg);

                        if write
                            .lock()
                            .await
                            .send(Message::Text(Utf8Bytes::from(sub.to_string())))
                            .await
                            .is_err()
                        {
                            METRICS.subscription_errors.fetch_add(1, Ordering::Relaxed);
                            METRICS
                                .ws_connections_active
                                .fetch_sub(1, Ordering::Relaxed);
                            break;
                        }

                        METRICS.subscriptions_sent.fetch_add(1, Ordering::Relaxed);
                    }
                }


                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            // ---- KUCOIN JSON PING HANDLING ----
                            if adapter.name() == "kucoin" {
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                                    if v.get("type").and_then(|t| t.as_str()) == Some("ping") {
                                        let pong = serde_json::json!({
                                            "type": "pong",
                                            "id": v.get("id")
                                        });

                                        let _ = write
                                            .lock()
                                            .await
                                            .send(Message::Text(Utf8Bytes::from(pong.to_string())))
                                            .await;

                                        // optional metrics
                                        // METRICS.pongs_sent.fetch_add(1, Ordering::Relaxed);
                                        continue;
                                    }
                                }
                            }

                            // ---- NORMAL MESSAGE FLOW ----
                            handle_parsed(
                                adapter.parse_message(&text, adapter.name()),
                                &master,
                            )
                                .await;
                        }

                        Ok(Message::Binary(bin)) => {
                            let mut decoder = flate2::read::GzDecoder::new(&bin[..]);
                            let mut decoded = String::new();

                            if decoder.read_to_string(&mut decoded).is_ok() {
                                handle_parsed(
                                    adapter.parse_message(&decoded, adapter.name()),
                                    &master,
                                )
                                    .await;
                            }
                        }

                        Ok(Message::Ping(p)) => {
                            let _ = write
                                .lock()
                                .await
                                .send(Message::Pong(p))
                                .await;
                        }

                        Ok(Message::Close(frame)) => {
                            eprintln!(
                                "[WS CLOSE][{}] {:?}",
                                adapter.name(),
                                frame
                            );
                            break;
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }

                METRICS
                    .ws_connections_active
                    .fetch_sub(1, Ordering::Relaxed);
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

        METRICS.ws_reconnects.fetch_add(1, Ordering::Relaxed);
        sleep(Duration::from_secs(5)).await;
    }
}

async fn handle_parsed(
    result: ParseResult,
    master: &MasterPool,
) {
    match result {
        ParseResult::Market(mm) => {
            METRICS.trades_received.fetch_add(1, Ordering::Relaxed);

            if master.send(serde_json::to_value(mm).unwrap()).await.is_ok() {
                METRICS.trades_forwarded.fetch_add(1, Ordering::Relaxed);
            } else {
                METRICS.send_errors.fetch_add(1, Ordering::Relaxed);
                METRICS.dropped_messages.fetch_add(1, Ordering::Relaxed);
            }
        }

        ParseResult::Control => {
            // optional:
            // METRICS.control_messages.fetch_add(1, Ordering::Relaxed);
        }

        ParseResult::Error => {
            METRICS.parse_errors.fetch_add(1, Ordering::Relaxed);
        }
    }
}
