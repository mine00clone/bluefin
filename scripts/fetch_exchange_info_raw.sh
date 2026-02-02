#!/usr/bin/env bash
set -euo pipefail

RUN_CFG="${1:-config/run/run_live.toml}"

cargo run --example raw_exchange_info_dump -p bf_rest -- --run "$RUN_CFG"
