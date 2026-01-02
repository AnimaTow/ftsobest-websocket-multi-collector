#!/usr/bin/env bash
set -euo pipefail

# -----------------------------
# CONFIG
# -----------------------------
FEEDS_URL="https://raw.githubusercontent.com/flare-foundation/ftso-v2-example-value-provider/main/src/config/feeds.json"
COINBASE_PRODUCTS_URL="https://api.exchange.coinbase.com/products"
QUOTES=("USD" "USDT")

TMP_DIR="$(mktemp -d)"
FEEDS_JSON="$TMP_DIR/feeds.json"
COINBASE_SYMBOLS="$TMP_DIR/coinbase_symbols.txt"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# -----------------------------
# FETCH DATA
# -----------------------------
echo "→ Lade feeds.json von GitHub"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Lade Coinbase products"
curl -fsSL "$COINBASE_PRODUCTS_URL" \
  | jq -r '.[]
      | select(.status=="online")
      | select(.trading_disabled==false)
      | "\(.base_currency)-\(.quote_currency)"' \
  > "$COINBASE_SYMBOLS"

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[COINBASE – SUPPORTED TRADE PAIRS]"
echo

jq -r '.[].feed.name' "$FEEDS_JSON" \
| cut -d/ -f1 \
| sort -u \
| while read -r BASE; do
    for QUOTE in "${QUOTES[@]}"; do
      SYMBOL="${BASE}-${QUOTE}"
      if grep -qx "$SYMBOL" "$COINBASE_SYMBOLS"; then
        echo "\"$BASE/$QUOTE\","
      fi
    done
  done
