# 005 - Market Snapshot

## Purpose
Store market-level constraints (tick/step/min/max) for all symbols. Used by executor normalization to avoid reject/cancel.

## Format
- JSON snapshot generated from raw /exchange/info.
- Must be committed to git and loaded at runtime.

## Update Policy
1. Fetch raw /exchange/info and save to data/raw/rest/exchange_info_YYYYMMDD_HHMMSS.json
2. Manually inspect the raw response
3. Generate config/markets/snapshot.json from the raw file
   - scripts/generate_market_snapshot.sh data/raw/rest/exchange_info_YYYYMMDD_HHMMSS.json

## Impact Scope
- Any change affects price/size rounding and minimums across all orders.
- Stale snapshot risks rejects or incorrect normalization.

## Writing Rules
- Do not edit snapshot.json by hand.
- Always save raw first, then generate.
- Keep fields aligned with actual API response (no assumptions).
