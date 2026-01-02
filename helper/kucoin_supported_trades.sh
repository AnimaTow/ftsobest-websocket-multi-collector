#!/usr/bin/env bash
set -euo pipefail

# -----------------------------
# CONFIG
# -----------------------------
FEEDS_URL="https://raw.githubusercontent.com/flare-foundation/ftso-v2-example-value-provider/main/src/config/feeds.json"
KUCOIN_SYMBOLS_URL="https://api.kucoin.com/api/v1/symbols"
QUOTES=("USD" "USDT")

TMP_DIR="$(mktemp -d)"
FEEDS_JSON="$TMP_DIR/feeds.json"
KUCOIN_SYMBOLS="$TMP_DIR/kucoin_symbols.txt"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# -----------------------------
# FETCH DATA
# -----------------------------
echo "→ Fetching feeds.json from GitHub"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Fetching KuCoin spot symbols"
curl -fsSL "$KUCOIN_SYMBOLS_URL" \
  | jq -r '
      .data[]
      | select(.enableTrading == true)
      | "\(.baseCurrency | ascii_upcase)-\(.quoteCurrency | ascii_upcase)"
    ' \
  > "$KUCOIN_SYMBOLS"

# -----------------------------
# BASIC VALIDATION
# -----------------------------
if [[ ! -s "$FEEDS_JSON" ]]; then
  echo "❌ feeds.json is empty"
  exit 1
fi

if [[ ! -s "$KUCOIN_SYMBOLS" ]]; then
  echo "❌ KuCoin symbol list is empty"
  exit 1
fi

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[KUCOIN – SUPPORTED TRADE PAIRS]"
echo

FOUND=0

while read -r BASE; do
  for QUOTE in "${QUOTES[@]}"; do
    SYMBOL="${BASE}-${QUOTE}"
    if grep -qx "$SYMBOL" "$KUCOIN_SYMBOLS"; then
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
  echo "❌ No supported KuCoin trade pairs found"
  exit 1
fi

echo
echo "✅ $FOUND supported KuCoin trade pairs found"
