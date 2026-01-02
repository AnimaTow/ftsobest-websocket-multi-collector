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
# FETCH DATA (always fresh)
# -----------------------------
echo "→ Lade feeds.json von GitHub"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Lade Gate.io Spot currency_pairs"
curl -fsSL "$GATEIO_PAIRS_URL" \
  | jq '.[] | select(.trade_status=="tradable") | "\(.base)_\(.quote)"' \
  > "$GATEIO_SYMBOLS"

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[GATE.IO – SUPPORTED TRADE PAIRS]"
echo

jq -r '.[].feed.name' "$FEEDS_JSON" \
| cut -d/ -f1 \
| sort -u \
| while read -r BASE; do
    for QUOTE in "${QUOTES[@]}"; do
      SYMBOL="${BASE}_${QUOTE}"
      if grep -qx "\"$SYMBOL\"" "$GATEIO_SYMBOLS"; then
        echo "\"$BASE/$QUOTE\","
      fi
    done
  done
