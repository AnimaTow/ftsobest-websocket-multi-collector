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
# FETCH DATA (always fresh)
# -----------------------------
echo "→ Lade feeds.json von GitHub"
curl -fsSL "$FEEDS_URL" -o "$FEEDS_JSON"

echo "→ Lade KuCoin Spot symbols"
curl -fsSL "$KUCOIN_SYMBOLS_URL" \
  | jq '.data[]
        | select(.enableTrading == true)
        | "\(.baseCurrency)-\(.quoteCurrency)"' \
  > "$KUCOIN_SYMBOLS"

# -----------------------------
# PROCESS
# -----------------------------
echo
echo "[KUCOIN – SUPPORTED TRADE PAIRS]"
echo

jq -r '.[].feed.name' "$FEEDS_JSON" \
| cut -d/ -f1 \
| sort -u \
| while read -r BASE; do
    for QUOTE in "${QUOTES[@]}"; do
      SYMBOL="${BASE}-${QUOTE}"
      if grep -qx "\"$SYMBOL\"" "$KUCOIN_SYMBOLS"; then
        echo "\"$BASE/$QUOTE\","
      fi
    done
  done
