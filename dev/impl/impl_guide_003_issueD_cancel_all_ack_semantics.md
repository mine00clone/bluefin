# impl_guide_003_issueD - cancel-all の返却曖昧性解消（空配列=成功? を禁止）

## Goal
- cancel-all（market 全キャンセル）の ACK 結果を「空配列」で返さない設計にする
- 呼び出し側が「空=キャンセル無し」と誤解して再注文/再キャンセルしないようにする

## Background / Risk
- `execute_cancel` は `AllForMarket` のとき `Ok(order_hashes.unwrap_or_default())` で空配列を返す
  - 該当: `crates/bf_order_exec/src/bluefin_executor.rs:229`
- trait `OrderExecutor::cancel -> Result<Vec<String>, CoreError>` が意味を表現しきれていない

## Scope
- Cancel の戻り値を「対象」と「ACK 受付」を明確に表現できる型に変更する案を採用する

## Files（追加/変更）
- MODIFY: `crates/bf_core/src/order.rs`（Cancel の domain 型追加）
- MODIFY: `crates/bf_core/src/traits.rs`（OrderExecutor::cancel の戻り値変更）
- MODIFY: `crates/bf_order_exec/src/bluefin_executor.rs`（実装追従）
- MODIFY: `crates/bf_rest/src/lib.rs`（trait 実装追従）

## Proposed API / Types
- 例: `bf_core::CancelAck`
  - `CancelAck::Single { market, order_hash }`
  - `CancelAck::Batch { market, order_hashes }`
  - `CancelAck::AllForMarket { market }`

`OrderExecutor::cancel` は `Result<CancelAck, CoreError>` を返し、cancel-all は `AllForMarket { market }` で表現する。

## Checklist
- [ ] cancel-all が空配列で返らない（型で表現される）
- [ ] 呼び出し側が cancel 対象を誤解しない（Single/Batch/All が分かる）
- [ ] 既存のキャンセル経路（examples/scripts）が破綻しないように追従修正される

## Verification
- `cargo test -p bf_core`（型/trait のコンパイル検証）
- `cargo test -p bf_order_exec`（cancel-all の戻り値が `AllForMarket` になる）
