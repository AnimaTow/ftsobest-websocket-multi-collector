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

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# -----------------------------
# FETCH DATA
# -----------------------------
echo "→ Fetching feeds.json from GitHub"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Fetching Bitrue exchangeInfo (spot trades)"
curl -fsSL "$BITRUE_EXCHANGEINFO_URL" \
| jq -r '
    .symbols[]
    | select(.status=="TRADING")
    | "\(.baseAsset | ascii_upcase)/\(.quoteAsset | ascii_upcase)"
  ' \
> "$BITRUE_SYMBOLS"

# -----------------------------
# BASIC VALIDATION
# -----------------------------
if [[ ! -s "$FEEDS_JSON" ]]; then
  echo "❌ feeds.json is empty"
  exit 1
fi

if [[ ! -s "$BITRUE_SYMBOLS" ]]; then
  echo "❌ Bitrue symbol list is empty"
  exit 1
fi

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[BITRUE – SUPPORTED TRADE PAIRS]"
echo

FOUND=0

while read -r BASE; do
  for QUOTE in "${QUOTES[@]}"; do
    SYMBOL="$BASE/$QUOTE"
    if grep -qx "$SYMBOL" "$BITRUE_SYMBOLS"; then
      echo "\"$SYMBOL\","
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
  echo "❌ No supported Bitrue trade pairs found"
  exit 1
fi

echo
echo "✅ $FOUND supported Bitrue trade pairs found"
