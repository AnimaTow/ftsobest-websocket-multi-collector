#!/usr/bin/env bash
set -euo pipefail

for script in helper/*_supported_trades.sh; do
  echo "::group::Running $script"
  chmod +x "$script"
  bash "$script"
  echo "::endgroup::"
done