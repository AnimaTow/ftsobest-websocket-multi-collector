```text
███████╗████████╗███████╗ ██████╗ ██████╗ ███████╗███████╗████████╗
██╔════╝╚══██╔══╝██╔════╝██╔═══██╗██╔══██╗██╔════╝██╔════╝╚══██╔══╝
█████╗     ██║   ███████╗██║   ██║██████╔╝█████╗  ███████╗   ██║   
██╔══╝     ██║   ╚════██║██║   ██║██╔══██╗██╔══╝  ╚════██║   ██║   
██║        ██║   ███████║╚██████╔╝██████╔╝███████╗███████║   ██║   
╚═╝        ╚═╝   ╚══════╝ ╚═════╝ ╚═════╝ ╚══════╝╚══════╝   ╚═╝   
        WebSocket Multi-Exchange Market Data Collector
            Built by AnimaTow : https://ftso.best
```
# FTSOBest WebSocket Multi Collector

A high-performance, multi-exchange WebSocket collector written in Rust.

This project connects to multiple cryptocurrency exchanges via WebSocket,
normalizes market data (trades, order books, tickers),
and forwards it to a central master service for aggregation, storage, and further processing.

The system is designed for:
- High throughput
- Low latency
- Exchange-agnostic extensibility
- Stable long-running operation

---

## Features

- Multi-exchange WebSocket collectors
- Unified market data schema
- Trade and order book support
- Per-exchange adapter architecture
- Config-driven symbol and connection management
- Automatic reconnect handling
- Optional demo mode (no master connection)
- Rustls-based TLS (rustls >= 0.23 compatible)

---

## Architecture Overview

```
Exchange WebSocket
        |
        v
ExchangeAdapter
        |
        v
Collector Runner
        |
        v
MasterPool
        |
        v
Master Service
```

---

## Supported Exchanges

Currently implemented:
- Gate.io
- Binance
- Binance US
- Coinbase
- OKX
- KuCoin
- Bitrue
- Bitfinix
- Bitstamp
- Kraken
- 
Planned:
- Mexc

---

## Configuration

All runtime behavior is controlled via `config.json`.

### Example

```json
{
  "master": {
    "url": "ws://127.0.0.1:8766",
    "connections": 5,
    "key": "YOUR_MASTER_KEY",
    "demo": false
  },
  "exchanges": [
    {
      "name": "gateio",
      "enabled": true,
      "pairs": {
        "trades": ["BTC/USDT", "ETH/USDT"],
        "orderbooks": ["BTC/USDT"]
      },
      "chunking": {
        "trades_per_connection": 10,
        "orderbooks_per_connection": 1
      },
      "orderbook": {
        "depth": 20,
        "update_interval_ms": 1000
      }
    }
  ]
}
```

---

## Running

```bash
cargo run
```

Production build:

```bash
cargo build --release
```

---

## License

License: Source Available – Non-Commercial  
Commercial use requires a separate license.
