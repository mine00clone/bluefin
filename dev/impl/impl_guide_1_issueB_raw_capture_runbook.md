002 - Raw Capture Runbook (Issue B)

## Purpose
Document the operational steps to capture raw REST/WS responses before any parsing or modeling.

## Related Files
- scripts/raw_auth_token.sh
- scripts/raw_open_orders.sh
- scripts/raw_account_details.sh
- scripts/raw_create_order.sh
- scripts/raw_cancel_orders.sh
- scripts/raw_ws_account.sh
- scripts/raw_ws_market.sh
- crates/bf_auth/examples/get_token.rs
- crates/bf_rest/examples/open_orders_raw.rs
- crates/bf_rest/examples/account_details_raw.rs
- crates/bf_rest/examples/create_order_raw.rs
- crates/bf_rest/examples/cancel_orders_raw.rs
- crates/bf_ws/examples/account_stream.rs
- crates/bf_ws/examples/market_stream.rs

## Runbook
### 1) Prerequisites
- Ensure `.env` contains:
  - BLUEFIN_PRIVATE_KEY (64-char hex)
  - BLUEFIN_ACCOUNT_ADDRESS
- Confirm `config/default.toml` has correct env URLs.
- Raw outputs will be written to `data/raw/` (gitignored).

### 2) REST raw capture
- Auth token (safe, masked output):
  - Run: `scripts/raw_auth_token.sh`
  - Output: `data/raw/rest/auth_token_YYYYMMDD_HHMMSS.json`

- Account details:
  - Run: `scripts/raw_account_details.sh`
  - Output: `data/raw/rest/account_details_YYYYMMDD_HHMMSS.json`

- Open orders:
  - Run: `scripts/raw_open_orders.sh`
  - Output: `data/raw/rest/open_orders_YYYYMMDD_HHMMSS.json`

- Create order (requires explicit opt-in):
  - Set: `BLUEFIN_ALLOW_TRADING=1`
  - Optional: `BLUEFIN_USE_TEST_KEYS=1` on staging
  - Params: `BLUEFIN_ORDER_MARKET`, `BLUEFIN_ORDER_SIDE`, `BLUEFIN_ORDER_TYPE`, `BLUEFIN_ORDER_SIZE`, `BLUEFIN_ORDER_PRICE`, `BLUEFIN_ORDER_TIF`
  - Run: `scripts/raw_create_order.sh`
  - Output: `data/raw/rest/create_order_YYYYMMDD_HHMMSS.json`

- Cancel orders (requires explicit opt-in):
  - Set: `BLUEFIN_ALLOW_TRADING=1`
  - Provide one of:
    - `BLUEFIN_CANCEL_ORDER_HASHES=hash1,hash2`
    - `BLUEFIN_CANCEL_ALL=1` with `BLUEFIN_CANCEL_MARKET`
  - Run: `scripts/raw_cancel_orders.sh`
  - Output: `data/raw/rest/cancel_orders_YYYYMMDD_HHMMSS.json`

### 3) WS raw capture
- Account stream:
  - Run: `scripts/raw_ws_account.sh`
  - Output: `data/raw/ws/account_stream_YYYYMMDD_HHMMSS.ndjson`

- Market stream:
  - Run: `scripts/raw_ws_market.sh`
  - Output: `data/raw/ws/market_stream_YYYYMMDD_HHMMSS.ndjson`

### 4) Manual inspection (mandatory)
- Open the saved raw files and confirm the true response shape.
- Only after confirming the shape should parsing or modeling begin.

### 5) Redaction policy (before sharing)
- Mask account addresses, order hashes, and any identifiers.
- Share only masked samples under `tests/fixtures/` if needed.

## Acceptance Criteria
- All scripts run without code changes.
- Raw outputs are saved under `data/raw/`.
- Manual inspection performed before any parsing work.

## Checklist
- [ ] Confirm `.env` and `config/default.toml`
- [ ] Capture REST raw responses
- [ ] Capture WS raw messages
- [ ] Manually inspect raw files
- [ ] Mask sensitive data before sharing
