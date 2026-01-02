#!/usr/bin/env bash
set -euo pipefail

# -----------------------------
# CONFIG
# -----------------------------
FEEDS_URL="https://raw.githubusercontent.com/flare-foundation/ftso-v2-example-value-provider/main/src/config/feeds.json"
BINANCE_US_EXCHANGEINFO_URL="https://api.binance.us/api/v3/exchangeInfo"
QUOTES=("USD" "USDT")

TMP_DIR="$(mktemp -d)"
FEEDS_JSON="$TMP_DIR/feeds.json"
BINANCE_SYMBOLS="$TMP_DIR/binanceus_symbols.txt"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# -----------------------------
# FETCH DATA
# -----------------------------
echo "→ Fetching feeds.json from GitHub"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Fetching Binance US exchangeInfo"
curl -fsSL "$BINANCE_US_EXCHANGEINFO_URL" \
  | jq '.symbols[] | select(.status=="TRADING") | .symbol' \
  > "$BINANCE_SYMBOLS"

# -----------------------------
# BASIC VALIDATION
# -----------------------------
if [[ ! -s "$FEEDS_JSON" ]]; then
  echo "❌ feeds.json is empty"
  exit 1
fi

if [[ ! -s "$BINANCE_SYMBOLS" ]]; then
  echo "❌ Binance US symbol list is empty"
  exit 1
fi

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[BINANCE US – SUPPORTED TRADE PAIRS]"
echo

FOUND=0

while read -r BASE; do
  for QUOTE in "${QUOTES[@]}"; do
    SYMBOL="${BASE}${QUOTE}"
    if grep -qx "\"$SYMBOL\"" "$BINANCE_SYMBOLS"; then
      echo "\"$BASE/$QUOTE\","
      FOUND=$((FOUND + 1))
    fi
  done
done < <(
  jq -r '.[].feed.name' "$FEEDS_JSON" \
  | cut -d/ -f1 \
  | sort -u
)

# -----------------------------
# FINAL ASSERTION (CI)
# -----------------------------
if [[ "$FOUND" -eq 0 ]]; then
  echo
  echo "❌ No supported Binance US trade pairs found"
  exit 1
fi

echo
echo "✅ $FOUND supported Binance US trade pairs found"
