use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use serde_json::Value;
use anyhow::Result;

use tokio::time::{Duration, sleep};
use tokio::sync::{mpsc, OnceCell, Mutex, Notify};

use rand::random_range;

/// ============================================================
/// MasterSender
/// ============================================================
///
/// Represents a **single persistent WebSocket connection** to the master.
///
/// Responsibilities:
/// - Perform login handshake
/// - Send JSON messages from an internal queue
/// - Send periodic JSON pings
/// - Detect connection loss
/// - Reconnect automatically
///
/// Design constraints:
/// - Must never block collectors
/// - Must tolerate master restarts
/// - Must drop data gracefully under backpressure
///
/// This component is intentionally stateful and reconnect-safe.
#[derive(Clone)]
pub struct MasterSender {
    /// Queue used by collectors to enqueue outgoing messages.
    ///
    /// This sender is replaced on every reconnect.
    pub queue: Arc<Mutex<mpsc::Sender<Value>>>,

    /// Signals that the connection has been established at least once.
    ///
    /// Used to prevent sending before the first successful login.
    connected: Arc<OnceCell<()>>,
}

impl MasterSender {

    /// Starts a persistent reconnect loop to the master.
    ///
    /// This function:
    /// - Initializes the shared queue
    /// - Spawns a background task
    /// - Reconnects indefinitely on errors
    ///
    /// CONTRACT:
    /// - This function never fails
    /// - All errors are handled internally
    pub async fn connect_loop(
        master_url: String,
        login_msg: String,
        debug: bool,
    ) -> Self {
        let (tx, _) = mpsc::channel::<Value>(10_000);
        let queue = Arc::new(Mutex::new(tx));
        let connected = Arc::new(OnceCell::new());

        let sender = Self {
            queue: queue.clone(),
            connected: connected.clone(),
        };

        // Background reconnect loop
        tokio::spawn({
            let queue = queue.clone();
            let connected = connected.clone();

            async move {
                loop {
                    // Create a fresh queue per connection
                    let (tx, rx) = mpsc::channel::<Value>(10_000);
                    {
                        let mut q = queue.lock().await;
                        *q = tx;
                    }

                    // Attempt to establish a WebSocket connection
                    if let Err(e) = Self::try_connect(
                        master_url.clone(),
                        login_msg.clone(),
                        debug,
                        rx,
                        connected.clone(),
                    ).await {
                        eprintln!("Master connection lost: {}", e);
                    }

                    // Backoff before reconnect
                    sleep(Duration::from_secs(30)).await;
                }
            }
        });

        sender
    }

    /// Establishes a single WebSocket connection to the master.
    ///
    /// This function:
    /// - Connects to the master WebSocket
    /// - Sends login message
    /// - Spawns a reader task
    /// - Runs the writer loop (queue + ping)
    ///
    /// TERMINATION:
    /// - Returns an error when the connection is closed
    async fn try_connect(
        master_url: String,
        login_msg: String,
        debug: bool,
        mut rx: mpsc::Receiver<Value>,
        connected: Arc<OnceCell<()>>,
    ) -> Result<()> {
        let (ws, _) = connect_async(&master_url).await?;
        let (mut write, mut read) = ws.split();

        // Used to notify the writer when the reader detects EOF
        let closed = Arc::new(Notify::new());

        // Mark connection as ready immediately after socket establishment
        // (matches behavior of legacy collectors)
        let _ = connected.set(());

        // ------------------------------------------------------------
        // LOGIN HANDSHAKE
        // ------------------------------------------------------------
        write.send(Message::Text(login_msg.clone().into())).await?;
        if debug {
            println!("Login message sent: {}", login_msg);
        }

        let mut ping_interval = tokio::time::interval(Duration::from_secs(30));

        // ------------------------------------------------------------
        // READER TASK
        // ------------------------------------------------------------
        // Purpose:
        // - Consume incoming messages (mostly ignored)
        // - Detect EOF / connection close
        // - Signal the writer to stop
        tokio::spawn({
            let closed = closed.clone();

            async move {
                while let Some(Ok(msg)) = read.next().await {
                    if let Message::Text(text) = msg {
                        if debug {
                            println!("[Master RECV] {}", text);
                        }
                    }
                }

                if debug {
                    println!("Master reader ended (EOF)");
                }

                closed.notify_waiters();
            }
        });

        // ------------------------------------------------------------
        // WRITER LOOP
        // ------------------------------------------------------------
        loop {
            tokio::select! {
                // Outgoing messages from collectors
                Some(msg) = rx.recv() => {
                    let json = serde_json::to_string(&msg)?;
                    if debug {
                        println!("[Master SEND] {}", json);
                    }
                    write.send(Message::Text(json.into())).await?;
                }

                // Periodic heartbeat
                _ = ping_interval.tick() => {
                    let ping = r#"{"op":"ping"}"#;
                    if debug {
                        println!("Master ping");
                    }
                    write.send(Message::Text(ping.into())).await?;
                }

                // Reader detected connection close
                _ = closed.notified() => {
                    if debug {
                        println!("Writer stopping: connection closed by master");
                    }
                    return Err(anyhow::anyhow!("Master closed connection"));
                }
            }
        }
    }

    /// Enqueues a message for sending to the master.
    ///
    /// Behavior:
    /// - Waits for initial connection
    /// - Uses non-blocking `try_send`
    /// - Drops messages if the queue is full
    ///
    /// This function must never block the caller.
    pub async fn send(&self, msg: Value) -> Result<()> {
        self.connected.get_or_init(|| async {}).await;

        let tx = self.queue.lock().await;
        match tx.try_send(msg) {
            Ok(_) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Send error: {}", e)),
        }
    }
}

/// ============================================================
/// MasterPool
/// ============================================================
///
/// Manages multiple `MasterSender` connections.
///
/// Purpose:
/// - Redundancy
/// - Load distribution
/// - Fault tolerance
///
/// Each sender maintains its own connection and queue.
#[allow(dead_code)]
pub struct MasterPool {
    senders: Vec<MasterSender>,
    counter: AtomicUsize,
    demo: bool,
}

impl MasterPool {

    /// Creates a pool of master connections.
    ///
    /// LOGIN FORMAT:
    /// - key=<API_KEY>&role=collector
    ///
    /// DEMO MODE:
    /// - No network connections
    /// - Messages are printed to stdout
    pub async fn new(
        master_url: String,
        login_msg: String,
        debug: bool,
        count: usize,
        demo: bool,
    ) -> Self {
        if demo {
            eprintln!("MasterPool running in DEMO mode");
        }

        let mut senders = Vec::with_capacity(count);

        if !demo {
            for _ in 0..count {
                let login = format!("key={}&role=collector", login_msg);
                let sender = MasterSender::connect_loop(
                    master_url.clone(),
                    login,
                    debug,
                ).await;
                senders.push(sender);
            }
        }

        Self {
            senders,
            counter: AtomicUsize::new(0),
            demo,
        }
    }

    /// Sends a message using a randomly selected sender.
    ///
    /// Behavior:
    /// - Up to 3 retry attempts
    /// - Backoff between retries
    /// - Fails gracefully if all senders are unavailable
    pub async fn send(&self, msg: Value) -> Result<()> {
        if self.demo {
            println!("DEMO → {}", serde_json::to_string(&msg)?);
            return Ok(());
        }

        for _ in 0..3 {
            let idx = random_range(0..self.senders.len());
            if self.senders[idx].send(msg.clone()).await.is_ok() {
                return Ok(());
            }
            sleep(Duration::from_millis(100)).await;
        }

        Err(anyhow::anyhow!("All master connections busy"))
    }
}

impl Clone for MasterPool {
    fn clone(&self) -> Self {
        Self {
            senders: self.senders.clone(),
            counter: AtomicUsize::new(self.counter.load(Ordering::Relaxed)),
            demo: self.demo,
        }
    }
}
