#!/usr/bin/env bash
set -euo pipefail

# -----------------------------
# CONFIG
# -----------------------------
FEEDS_URL="https://raw.githubusercontent.com/flare-foundation/ftso-v2-example-value-provider/main/src/config/feeds.json"
BYBIT_URL="https://api.bybit.com/v5/market/instruments-info?category=spot"
QUOTES=("USD" "USDT")

TMP_DIR="$(mktemp -d)"
FEEDS_JSON="$TMP_DIR/feeds.json"
BYBIT_SYMBOLS="$TMP_DIR/bybit_symbols.txt"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# -----------------------------
# FETCH DATA
# -----------------------------
echo "→ Fetch feeds.json"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Fetch Bybit spot instruments"
curl -fsSL "$BYBIT_URL" \
  | jq -r '.result.list[]
           | select(.status=="Trading")
           | "\(.baseCoin)/\(.quoteCoin)"' \
  > "$BYBIT_SYMBOLS"

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[BYBIT – SUPPORTED TRADE PAIRS]"
echo

jq -r '.[].feed.name' "$FEEDS_JSON" \
| cut -d/ -f1 \
| sort -u \
| while read -r BASE; do
    for QUOTE in "${QUOTES[@]}"; do
      PAIR="$BASE/$QUOTE"
      if grep -qx "$PAIR" "$BYBIT_SYMBOLS"; then
        echo "\"$PAIR\","
      fi
    done
  done
