/// Utility helpers used by all collectors.
///
/// This module contains:
/// - Symbol normalization helpers
/// - Time helpers
/// - Small format conversion utilities
///
/// IMPORTANT:
/// - No exchange-specific business logic should live here.
/// - This module must remain lightweight and deterministic.
///
/// Exchange-specific behavior should be handled in:
/// - adapters
/// - exchange configuration
/// - adapter implementations
///

use std::time::{SystemTime, UNIX_EPOCH};

/// Normalize trading symbols into the internal master format.
///
/// Target format:
///     BASE/QUOTE
///
/// Examples:
/// - "BTC_USDT"  -> "BTC/USDT"
/// - "ETH-USDT"  -> "ETH/USDT"
/// - "XRP/USD"   -> "XRP/USD"
///
/// DESIGN NOTES:
/// - This function is intentionally simple and generic.
/// - It performs only character-based normalization.
/// - Validation of symbol correctness happens elsewhere.
///
/// TODO:
/// - Consider enforcing uppercase symbols.
/// - Consider validating BASE/QUOTE structure.
///
#[allow(dead_code)]
pub fn normalize_symbol(raw: &str) -> String {
    raw.replace('_', "/").replace('-', "/")
}

/// Returns the current Unix timestamp in milliseconds.
///
/// This function is used across the collector pipeline for:
/// - Trade timestamps
/// - Orderbook update timestamps
/// - Heartbeat / event timing
///
/// PANIC:
/// - Panics if system time is before UNIX_EPOCH (should never happen).
///
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is before UNIX_EPOCH")
        .as_millis() as i64
}

/// Convert an internal symbol into the exchange-specific format.
///
/// Input:
/// - exchange: exchange identifier (e.g. "gateio", "binance")
/// - symbol: internal format "BASE/QUOTE"
///
/// Output:
/// - Exchange-specific symbol representation
///
/// Examples:
/// - ("gateio", "BTC/USDT")   -> "BTC_USDT"
/// - ("binance", "BTC/USDT")  -> "BTCUSDT"
/// - ("coinbase", "BTC/USDT") -> "BTC-USDT"
///
/// DESIGN NOTES:
/// - Centralized symbol conversion avoids duplication across adapters.
/// - Keeps configuration files exchange-agnostic.
///
/// TODO:
/// - Replace string-based exchange matching with enum-based dispatch.
/// - Add unit tests per exchange.
///
pub fn symbol_to_exchange(exchange: &str, symbol: &str) -> String {
    match exchange {
        "gateio" | "bitrue" => symbol.replace('/', "_"),

        "binance" | "binanceus" => symbol.replace('/', ""),

        "okx" | "kucoin" | "coinbase" => symbol.replace('/', "-"),

        _ => symbol.to_string(),
    }
}

/// Convert an exchange-specific symbol into the internal format.
///
/// Input:
/// - exchange: exchange identifier
/// - symbol: exchange-specific symbol
///
/// Output:
/// - Internal normalized format "BASE/QUOTE"
///
/// Examples:
/// - ("gateio", "BTC_USDT")   -> "BTC/USDT"
/// - ("coinbase", "BTC-USDT") -> "BTC/USDT"
///
/// IMPORTANT:
/// - Some exchanges (notably Binance) do not provide explicit
///   separators in their symbols.
///
/// TODO:
/// - Implement proper base/quote detection for Binance symbols
///   using known quote assets (USDT, USD, BTC, etc.).
/// - Move complex parsing into dedicated exchange adapters.
///
pub fn symbol_from_exchange(exchange: &str, symbol: &str) -> String {
    match exchange {
        "gateio" | "bitrue" => symbol.replace('_', "/"),

        "binance" | "binanceus" => {
            // Binance symbols do not include a separator.
            // This is intentionally left as-is for now.
            //
            // Example: "BTCUSDT" -> requires external logic
            // to split into BASE/QUOTE.
            symbol.to_string()
        }

        "okx" | "kucoin" | "coinbase" => symbol.replace('-', "/"),

        _ => symbol.to_string(),
    }
}
