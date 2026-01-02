#!/usr/bin/env bash
set -euo pipefail

# -----------------------------
# CONFIG
# -----------------------------
FEEDS_URL="https://raw.githubusercontent.com/flare-foundation/ftso-v2-example-value-provider/main/src/config/feeds.json"
GATEIO_PAIRS_URL="https://api.gateio.ws/api/v4/spot/currency_pairs"
QUOTES=("USD" "USDT")

TMP_DIR="$(mktemp -d)"
FEEDS_JSON="$TMP_DIR/feeds.json"
GATEIO_SYMBOLS="$TMP_DIR/gateio_symbols.txt"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# -----------------------------
# FETCH DATA
# -----------------------------
echo "→ Fetching feeds.json from GitHub"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Fetching Gate.io spot currency pairs"
curl -fsSL "$GATEIO_PAIRS_URL" \
  | jq -r '
      .[]
      | select(.trade_status=="tradable")
      | "\(.base | ascii_upcase)_\(.quote | ascii_upcase)"
    ' \
  > "$GATEIO_SYMBOLS"

# -----------------------------
# BASIC VALIDATION
# -----------------------------
if [[ ! -s "$FEEDS_JSON" ]]; then
  echo "❌ feeds.json is empty"
  exit 1
fi

if [[ ! -s "$GATEIO_SYMBOLS" ]]; then
  echo "❌ Gate.io symbol list is empty"
  exit 1
fi

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[GATE.IO – SUPPORTED TRADE PAIRS]"
echo

FOUND=0

while read -r BASE; do
  for QUOTE in "${QUOTES[@]}"; do
    SYMBOL="${BASE}_${QUOTE}"
    if grep -qx "$SYMBOL" "$GATEIO_SYMBOLS"; then
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
  echo "❌ No supported Gate.io trade pairs found"
  exit 1
fi

echo
echo "✅ $FOUND supported Gate.io trade pairs found"
