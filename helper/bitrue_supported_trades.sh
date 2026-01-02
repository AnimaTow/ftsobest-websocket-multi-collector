#!/usr/bin/env bash
set -euo pipefail

# -----------------------------
# CONFIG
# -----------------------------
FEEDS_URL="https://raw.githubusercontent.com/flare-foundation/ftso-v2-example-value-provider/main/src/config/feeds.json"
BITRUE_EXCHANGEINFO_URL="https://www.bitrue.com/api/v1/exchangeInfo"
QUOTES=("USD" "USDT")

TMP_DIR="$(mktemp -d)"
FEEDS_JSON="$TMP_DIR/feeds.json"
BITRUE_SYMBOLS="$TMP_DIR/bitrue_symbols.txt"

trap 'rm -rf "$TMP_DIR"' EXIT

# -----------------------------
# FETCH DATA
# -----------------------------
echo "→ Lade feeds.json"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Lade Bitrue exchangeInfo (Spot Trades)"
curl -fsSL "$BITRUE_EXCHANGEINFO_URL" \
| jq -r '
    .symbols[]
    | select(.status=="TRADING")
    | "\(.baseAsset | ascii_upcase)/\(.quoteAsset | ascii_upcase)"
  ' \
> "$BITRUE_SYMBOLS"

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[BITRUE – SUPPORTED *TRADE* PAIRS]"
echo

jq -r '.[].feed.name' "$FEEDS_JSON" \
| cut -d/ -f1 \
| sort -u \
| while read -r BASE; do
    for QUOTE in "${QUOTES[@]}"; do
      SYMBOL="$BASE/$QUOTE"
      if grep -qx "$SYMBOL" "$BITRUE_SYMBOLS"; then
        echo "\"$SYMBOL\","
      fi
    done
  done
