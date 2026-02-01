#!/usr/bin/env bash
set -euo pipefail

BLUEFIN_ALLOW_TRADING=1 cargo run --example create_order_raw -p bf_rest
