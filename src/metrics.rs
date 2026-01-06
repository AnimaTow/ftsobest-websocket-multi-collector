use std::sync::atomic::{AtomicUsize};
use std::sync::Arc;

use once_cell::sync::Lazy;

/// Global runtime metrics for the collector.
///
/// Purpose:
/// - Track active exchanges
/// - Track WebSocket connections
/// - Track active markets (pairs)
/// - Track throughput (received / forwarded messages)
///
/// Design:
/// - Lock-free (Atomics)
/// - Cheap to update
/// - Safe in async + multithreaded contexts
#[derive(Default)]
pub struct RuntimeMetrics {
    // High-level
    pub exchanges_active: AtomicUsize,

    // WebSocket level
    pub ws_connections_active: AtomicUsize,

    // Markets
    pub trade_pairs_active: AtomicUsize,
    pub orderbook_pairs_active: AtomicUsize,

    // Throughput
    pub trades_received: AtomicUsize,
    pub trades_forwarded: AtomicUsize,

    pub parse_errors: AtomicUsize,
    pub send_errors: AtomicUsize,
    pub ws_reconnects: AtomicUsize,
    pub dropped_messages: AtomicUsize,

    pub subscriptions_sent: AtomicUsize,
    pub subscription_errors: AtomicUsize,
}

/// Global metrics registry (singleton)
pub static METRICS: Lazy<Arc<RuntimeMetrics>> =
    Lazy::new(|| Arc::new(RuntimeMetrics::default()));
