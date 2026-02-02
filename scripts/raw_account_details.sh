#!/usr/bin/env bash
set -euo pipefail

RUN_CFG="${1:-config/run/run_live.toml}"

cargo run --example account_details_raw -p bf_rest -- --run "$RUN_CFG"
