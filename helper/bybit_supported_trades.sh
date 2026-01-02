#!/usr/bin/env bash
set -euo pipefail

# -----------------------------
# CONFIG
# -----------------------------
FEEDS_URL="https://raw.githubusercontent.com/flare-foundation/ftso-v2-example-value-provider/main/src/config/feeds.json"
BYBIT_EXCHANGEINFO_URL="https://api.bybit.com/v5/market/instruments-info?category=spot"
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
echo "→ Fetching feeds.json from GitHub"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Fetching Bybit spot instruments"
curl -fsSL "$BYBIT_EXCHANGEINFO_URL" \
  | jq -r '
      .result.list[]
      | select(.status=="Trading")
      | "\(.baseCoin | ascii_upcase)/\(.quoteCoin | ascii_upcase)"
    ' \
  > "$BYBIT_SYMBOLS"

# -----------------------------
# BASIC VALIDATION
# -----------------------------
if [[ ! -s "$FEEDS_JSON" ]]; then
  echo "❌ feeds.json is empty"
  exit 1
fi

if [[ ! -s "$BYBIT_SYMBOLS" ]]; then
  echo "❌ Bybit symbol list is empty"
  exit 1
fi

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[BYBIT – SUPPORTED TRADE PAIRS]"
echo

FOUND=0

while read -r BASE; do
  for QUOTE in "${QUOTES[@]}"; do
    PAIR="$BASE/$QUOTE"
    if grep -qx "$PAIR" "$BYBIT_SYMBOLS"; then
      echo "\"$PAIR\","
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
  echo "❌ No supported Bybit trade pairs found"
  exit 1
fi

echo
echo "✅ $FOUND supported Bybit trade pairs found"
