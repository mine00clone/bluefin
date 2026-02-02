# 004 - Plans

## Purpose
Define order plans as data (multiple orders, multiple markets) to enable strategy execution without code changes.

## Format
- TOML with [[orders]] array entries.
- price/size must be string or integer (no floats).
- Price modes: absolute or offset_bps (relative to reference price).

## Impact Scope
- Plan changes directly affect created orders.
- Invalid plans should fail fast at startup (validation against market snapshot).

## Writing Rules
- Keep IDs stable for traceability (client_order_id).
- Use e9 quantities/price strings to avoid precision loss.
- Do not encode secrets or environment URLs here.
