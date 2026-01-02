#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SELF="$(basename "${BASH_SOURCE[0]}")"

shopt -s nullglob

SCRIPTS=("$SCRIPT_DIR"/*_supported_trades.sh)

for script in "${SCRIPTS[@]}"; do
  if [[ "$(basename "$script")" == "$SELF" ]]; then
    continue
  fi

  echo "::group::Running $(basename "$script")"
  chmod +x "$script"
  bash "$script"
  echo "::endgroup::"
done