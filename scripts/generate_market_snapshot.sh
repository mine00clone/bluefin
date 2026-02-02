#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <raw_exchange_info.json> [output_path]" >&2
  exit 1
fi

INPUT="$1"
OUTPUT="${2:-config/markets/snapshot.json}"

cargo run --example generate_market_snapshot -p bf_rest -- --input "$INPUT" --output "$OUTPUT"
