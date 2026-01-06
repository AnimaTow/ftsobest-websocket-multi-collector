#!/usr/bin/env bash
set -euo pipefail

# -----------------------------
# CONFIG
# -----------------------------
FEEDS_URL="https://raw.githubusercontent.com/flare-foundation/ftso-v2-example-value-provider/main/src/config/feeds.json"
BITFINEX_SYMBOLS_URL="https://api-pub.bitfinex.com/v2/tickers?symbols=ALL"

QUOTES=("USD" "USDT")

TMP_DIR="$(mktemp -d)"
FEEDS_JSON="$TMP_DIR/feeds.json"
BITFINEX_SYMBOLS="$TMP_DIR/bitfinex_symbols.txt"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# -----------------------------
# FETCH DATA
# -----------------------------
echo "→ Fetching feeds.json from GitHub"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Fetching Bitfinex tickers (ALL)"
curl -fsSL "$BITFINEX_SYMBOLS_URL" \
  | jq -r '
      .[]
      | select(type=="array")
      | .[0]
      | select(startswith("t"))
      | sub("^t"; "")
    ' \
  > "$BITFINEX_SYMBOLS"

# -----------------------------
# BASIC VALIDATION
# -----------------------------
if [[ ! -s "$FEEDS_JSON" ]]; then
  echo "❌ feeds.json is empty"
  exit 1
fi

if [[ ! -s "$BITFINEX_SYMBOLS" ]]; then
  echo "❌ Bitfinex symbol list is empty"
  exit 1
fi

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[BITFINEX – SUPPORTED TRADE PAIRS]"
echo

FOUND=0

while read -r BASE; do
  for QUOTE in "${QUOTES[@]}"; do
    SYMBOL="${BASE}${QUOTE}"
    if grep -qx "$SYMBOL" "$BITFINEX_SYMBOLS"; then
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
  echo "❌ No supported Bitfinex trade pairs found"
  exit 1
fi

echo
echo "✅ $FOUND supported Bitfinex trade pairs found"
