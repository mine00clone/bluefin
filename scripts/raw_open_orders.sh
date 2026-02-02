#!/usr/bin/env bash
set -euo pipefail

RUN_CFG="${1:-config/run/run_live.toml}"

cargo run --example open_orders_raw -p bf_rest -- --run "$RUN_CFG"
