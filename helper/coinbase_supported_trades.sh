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
echo "→ Fetching feeds.json from GitHub"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Fetching Coinbase products"
curl -fsSL "$COINBASE_PRODUCTS_URL" \
  | jq -r '
      .[]
      | select(.status=="online")
      | select(.trading_disabled==false)
      | "\(.base_currency | ascii_upcase)-\(.quote_currency | ascii_upcase)"
    ' \
  > "$COINBASE_SYMBOLS"

# -----------------------------
# BASIC VALIDATION
# -----------------------------
if [[ ! -s "$FEEDS_JSON" ]]; then
  echo "❌ feeds.json is empty"
  exit 1
fi

if [[ ! -s "$COINBASE_SYMBOLS" ]]; then
  echo "❌ Coinbase symbol list is empty"
  exit 1
fi

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[COINBASE – SUPPORTED TRADE PAIRS]"
echo

FOUND=0

while read -r BASE; do
  for QUOTE in "${QUOTES[@]}"; do
    SYMBOL="${BASE}-${QUOTE}"
    if grep -qx "$SYMBOL" "$COINBASE_SYMBOLS"; then
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
  echo "❌ No supported Coinbase trade pairs found"
  exit 1
fi

echo
echo "✅ $FOUND supported Coinbase trade pairs found"
