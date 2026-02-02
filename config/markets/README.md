# Market Snapshot

This file stores market-level constraints (tick/step/min/max) for all symbols.

## Update policy
1. Fetch raw /exchange/info and save to data/raw/rest/exchange_info_YYYYMMDD_HHMMSS.json
2. Manually inspect the raw response
3. Generate config/markets/snapshot.json from the raw file
   - scripts/generate_market_snapshot.sh data/raw/rest/exchange_info_YYYYMMDD_HHMMSS.json

Do not update snapshot.json without saving raw data and reviewing it.
