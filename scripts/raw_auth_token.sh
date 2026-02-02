#!/usr/bin/env bash
set -euo pipefail

RUN_CFG="${1:-config/run/run_live.toml}"

cargo run --example get_token -p bf_auth -- --run "$RUN_CFG"
