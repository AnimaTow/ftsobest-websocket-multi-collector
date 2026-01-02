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
# FETCH DATA (always fresh)
# -----------------------------
echo "→ Lade feeds.json von GitHub"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Lade OKX Spot instruments"
curl -fsSL "$OKX_INSTRUMENTS_URL" \
  | jq '.data[]
        | select(.state=="live")
        | "\(.baseCcy)-\(.quoteCcy)"' \
  > "$OKX_SYMBOLS"

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[OKX – SUPPORTED TRADE PAIRS]"
echo

jq -r '.[].feed.name' "$FEEDS_JSON" \
| cut -d/ -f1 \
| sort -u \
| while read -r BASE; do
    for QUOTE in "${QUOTES[@]}"; do
      SYMBOL="${BASE}-${QUOTE}"
      if grep -qx "\"$SYMBOL\"" "$OKX_SYMBOLS"; then
        echo "\"$BASE/$QUOTE\","
      fi
    done
  done
