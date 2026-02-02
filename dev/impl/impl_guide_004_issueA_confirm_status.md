# impl_guide_004_issueA - Confirm ステータス詳細化

## Goal
- Confirm の結果を詳細ステータスで返す
  - Active / Filled / PartiallyFilled / Canceled / Expired / TimedOut

## Files（追加/変更）
- ADD: `crates/bf_core/src/confirm.rs`
- MODIFY: `crates/bf_core/src/lib.rs`
- MODIFY: `crates/bf_order_manager`（WSイベントから Confirm 状態へ変換）

## Checklist
- [x] ConfirmStatus enum が追加される
- [x] WS OrderUpdate を ConfirmStatus にマップできる
- [x] TimedOut を明示的に扱える
