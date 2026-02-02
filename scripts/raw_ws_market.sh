#!/usr/bin/env bash
set -euo pipefail

RUN_CFG="${1:-config/run/run_live.toml}"

cargo run --example market_stream -p bf_ws -- --run "$RUN_CFG"
