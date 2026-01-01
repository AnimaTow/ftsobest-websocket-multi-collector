/// Collector module
///
/// This module groups all logic responsible for:
/// - Starting exchange collectors
/// - Managing WebSocket lifecycles per exchange
/// - Routing parsed market data to the master
///
/// The collector layer acts as the orchestration layer between:
/// - Exchange adapters (Gate.io, Binance, OKX, …)
/// - The MasterPool (output / aggregation layer)
///
/// Design notes:
/// - Exchange-specific logic MUST NOT live here
/// - This module should remain thin and orchestration-focused
/// - All heavy logic belongs to adapters or runner submodules
///
/// TODO:
/// - Add shared collector metrics (connections, reconnects, errors)
/// - Add optional rate-limit / backoff coordination across collectors
pub mod runner;
