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
        "gateio" => symbol.replace('/', "_"),
        "bitrue" => symbol.replace('/', "").to_lowercase(),
        "binance" | "binanceus" | "bybit" => symbol.replace('/', ""),
        "okx" | "kucoin" | "coinbase" => symbol.replace('/', "-"),
        "mexc" => symbol.replace('/', "_"),
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
///
const BINANCE_QUOTES: [&str; 2] = [
    "USDT",
    "USD"
];
pub fn symbol_from_exchange(exchange: &str, symbol: &str) -> String {
    match exchange {
        "gateio" => symbol.replace('_', "/"),
        "mexc" => symbol.replace('_', "/"),
        "binance" | "binanceus" => {
            for quote in BINANCE_QUOTES {
                if symbol.ends_with(quote) {
                    let base = &symbol[..symbol.len() - quote.len()];
                    if !base.is_empty() {
                        return format!("{}/{}", base, quote);
                    }
                }
            }

            // Fallback – should never happen for valid config symbols
            symbol.to_string()
        },
        "okx" | "kucoin" | "coinbase" => symbol.replace('-', "/"),
        "bybit" => {
            for quote in ["USDT", "USD"] {
                if symbol.ends_with(quote) {
                    let base = &symbol[..symbol.len() - quote.len()];
                    return format!("{}/{}", base, quote);
                }
            }
            symbol.to_string()
        },
        "bitrue" => {
            if symbol.ends_with("usdt") {
                format!("{}/USDT", symbol[..symbol.len() - 4].to_uppercase())
            } else if symbol.ends_with("usd") {
                format!("{}/USD", symbol[..symbol.len() - 3].to_uppercase())
            } else {
                symbol.to_uppercase()
            }
        },
        _ => symbol.to_string(),
    }
}
