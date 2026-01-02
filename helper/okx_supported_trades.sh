#!/usr/bin/env bash
set -euo pipefail

# -----------------------------
# CONFIG
# -----------------------------
FEEDS_URL="https://raw.githubusercontent.com/flare-foundation/ftso-v2-example-value-provider/main/src/config/feeds.json"
OKX_INSTRUMENTS_URL="https://www.okx.com/api/v5/public/instruments?instType=SPOT"
QUOTES=("USD" "USDT")

TMP_DIR="$(mktemp -d)"
FEEDS_JSON="$TMP_DIR/feeds.json"
OKX_SYMBOLS="$TMP_DIR/okx_symbols.txt"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# -----------------------------
# FETCH DATA
# -----------------------------
echo "→ Fetching feeds.json from GitHub"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Fetching OKX spot instruments"
curl -fsSL "$OKX_INSTRUMENTS_URL" \
  | jq -r '
      .data[]
      | select(.state=="live")
      | "\(.baseCcy | ascii_upcase)-\(.quoteCcy | ascii_upcase)"
    ' \
  > "$OKX_SYMBOLS"

# -----------------------------
# BASIC VALIDATION
# -----------------------------
if [[ ! -s "$FEEDS_JSON" ]]; then
  echo "❌ feeds.json is empty"
  exit 1
fi

if [[ ! -s "$OKX_SYMBOLS" ]]; then
  echo "❌ OKX symbol list is empty"
  exit 1
fi

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[OKX – SUPPORTED TRADE PAIRS]"
echo

FOUND=0

while read -r BASE; do
  for QUOTE in "${QUOTES[@]}"; do
    SYMBOL="${BASE}-${QUOTE}"
    if grep -qx "$SYMBOL" "$OKX_SYMBOLS"; then
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
  echo "❌ No supported OKX trade pairs found"
  exit 1
fi

echo
echo "✅ $FOUND supported OKX trade pairs found"
