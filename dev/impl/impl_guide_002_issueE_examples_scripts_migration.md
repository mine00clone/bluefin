# impl_guide_002_issueE - examples/scripts の更新（運用に耐える形へ）

## Goal
- 現行の単発 raw スクリプトを、RunConfig + Snapshot + Plan 駆動に置き換える
- 「実行時 market fetch」を撤廃し、必ず snapshot 参照
- raw保存・redaction を標準化

## Files（追加/変更）
- MODIFY: crates/bf_rest/examples/create_order_raw.rs
- MODIFY: crates/bf_rest/examples/cancel_order_raw.rs
- ADD: crates/bf_rest/examples/open_orders_raw.rs
- ADD: crates/bf_ws/examples/ws_account_raw_dump.rs
- ADD: crates/bf_ws/examples/ws_market_raw_dump.rs
- ADD: crates/bf_core/src/redact.rs（JSON redaction）
- MODIFY: scripts/*.sh（run config 引数を渡す）

## Checklist
- [x] `--run config/run/run_live.toml` の指定だけで examples が動く（raw系/WS系）
- [x] 生成した raw は data/raw に保存される（gitignore）
- [ ] ログ出力に order id / account info が残らない（redaction）
