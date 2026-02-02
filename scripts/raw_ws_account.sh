#!/usr/bin/env bash
set -euo pipefail

RUN_CFG="${1:-config/run/run_live.toml}"

cargo run --example account_stream -p bf_ws -- --run "$RUN_CFG"
