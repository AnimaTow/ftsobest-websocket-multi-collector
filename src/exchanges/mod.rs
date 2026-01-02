//! Exchange adapter registry and factory
//!
//! This module provides:
//! - Central registration of all supported exchanges
//! - A factory function to resolve adapters by name
//!
//! All exchange-specific logic must live in dedicated adapter modules.
//! The rest of the application must interact exclusively through
//! the `ExchangeAdapter` trait.

pub mod adapter;
pub mod gateio;
pub mod binanceus;
pub mod binance;
pub mod okx;
mod bitrue;
mod kucoin;
mod coinbase;
mod bybit;
mod mexc;

use std::sync::Arc;
use adapter::ExchangeAdapter;

/// Returns an exchange adapter instance by name.
///
/// This function acts as a **central factory / registry** for all
/// supported exchanges.
///
/// DESIGN:
/// - Keeps adapter creation in one place
/// - Avoids string-based logic scattered across the codebase
/// - Enables compile-time visibility of supported exchanges
///
/// PARAMETERS:
/// - `name`: Exchange identifier from configuration
///
/// RETURNS:
/// - `Some(Arc<dyn ExchangeAdapter>)` if the exchange is supported
/// - `None` if the exchange is unknown or disabled
///
/// CONTRACT:
/// - `name` MUST match the `exchange.name` field in config.json
/// - Adapter names must be lowercase and stable
///
/// THREADING:
/// - Adapters are wrapped in `Arc`
/// - The same adapter instance may be shared across tasks
///
/// TODO:
/// - Replace match-based registry with a static map if exchange
///   count grows significantly
/// - Add feature-flag based compilation per exchange
/// - Add optional runtime validation for duplicate names
///
pub fn get_adapter(name: &str) -> Option<Arc<dyn ExchangeAdapter>> {
    match name {
        "gateio" => Some(Arc::new(gateio::GateIoAdapter)),
        "binanceus" => Some(Arc::new(binanceus::BinanceUsAdapter)),
        "binance" => Some(Arc::new(binance::BinanceAdapter)),
        "okx"     => Some(Arc::new(okx::OkxAdapter)),
        "bitrue"     => Some(Arc::new(bitrue::BitrueAdapter)),
        "kucoin" => Some(Arc::new(kucoin::KucoinAdapter)),
        "coinbase" => Some(Arc::new(coinbase::CoinbaseAdapter)),
        "bybit" => Some(Arc::new(bybit::BybitAdapter)),
        _ => None,
    }
}
