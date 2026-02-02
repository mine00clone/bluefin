# impl_guide_003_issueE - OMS の不正遷移ハンドリング（warn 継続を見直す）

## Goal
- WS 由来の更新で不正遷移が来た場合に「安全側停止」できる設計にする
- 不正遷移を warn で流して状態を上書きしない（重複注文/誤認識の温床を排除）

## Background / Risk
- 現状は不正遷移を warn のみにして state を上書きする
  - 該当: `crates/bf_order_manager/src/lib.rs:69`

## Scope
- `OrderManager::apply_update` の不正遷移時の方針を決め、呼び出し側に伝播する

## Files（追加/変更）
- MODIFY: `crates/bf_order_manager/src/lib.rs`
- (必要なら) MODIFY: `crates/bf_core/src/error.rs`（遷移不整合エラーの統一）
- (必要なら) MODIFY: `crates/bf_app/src/main.rs`（不整合時の停止/再同期のハンドリング）

## Implementation Notes
- 方針案（どれを採用するか決める）:
  - Strict: 不正遷移なら `Err(OmsError::InvalidTransition{..})` を返し、上位が取引停止/再同期
  - Quarantine: 不正遷移の order を隔離し、reconcile まで更新停止（上位に signal）
- いずれも「勝手に注文を出す」方向のフォールバックは禁止

## Checklist
- [x] 不正遷移を検出したら、上位に伝播できる（Err or signal）
- [ ] 上位（将来の orchestration）が取引停止/再同期に繋げられる導線がある
- [x] 正常系の遷移は従来通り動く

## Verification
- `cargo test -p bf_order_manager`
- 不正遷移ケースのユニットテスト追加は別途検討
