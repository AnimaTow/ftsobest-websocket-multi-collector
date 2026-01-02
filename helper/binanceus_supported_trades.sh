#!/usr/bin/env bash
set -euo pipefail

# -----------------------------
# CONFIG
# -----------------------------
FEEDS_URL="https://raw.githubusercontent.com/flare-foundation/ftso-v2-example-value-provider/main/src/config/feeds.json"
BINANCE_EXCHANGEINFO_URL="https://api.binance.us/api/v3/exchangeInfo"
QUOTES=("USD" "USDT")

TMP_DIR="$(mktemp -d)"
FEEDS_JSON="$TMP_DIR/feeds.json"
BINANCE_SYMBOLS="$TMP_DIR/binanceus_symbols.txt"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# -----------------------------
# FETCH DATA (always fresh)
# -----------------------------
echo "→ Lade feeds.json von GitHub"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Lade Binance US exchangeInfo"
curl -fsSL "$BINANCE_EXCHANGEINFO_URL" \
  | jq '.symbols[] | select(.status=="TRADING") | .symbol' \
  > "$BINANCE_SYMBOLS"

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[BINANCE US – SUPPORTED TRADE PAIRS]"
echo

jq -r '.[].feed.name' "$FEEDS_JSON" \
| cut -d/ -f1 \
| sort -u \
| while read -r BASE; do
    for QUOTE in "${QUOTES[@]}"; do
      SYMBOL="${BASE}${QUOTE}"
      if grep -qx "\"$SYMBOL\"" "$BINANCE_SYMBOLS"; then
        echo "\"$BASE/$QUOTE\","
      fi
    done
  done
