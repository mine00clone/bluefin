# impl_guide_004_issueB - Normalize→Send→Confirm パイプライン統一

## Goal
- Executor が Normalize→Send→Confirm を一貫して担う
- Confirm は WS から確定し、REST は ACK のみ

## Files（追加/変更）
- MODIFY: `crates/bf_order_exec/src/lib.rs`
- MODIFY: `crates/bf_order_exec/src/normalize.rs`
- MODIFY: `crates/bf_ws`（Confirm 用のイベント取得導線）

## Checklist
- [ ] Normalize→Send→Confirm の順序が一貫する
- [ ] Confirm は WS 由来のみ
- [ ] ACK だけで成功扱いしない

