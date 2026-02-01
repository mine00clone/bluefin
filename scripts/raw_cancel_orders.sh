#!/usr/bin/env bash
set -euo pipefail

BLUEFIN_ALLOW_TRADING=1 cargo run --example cancel_orders_raw -p bf_rest
