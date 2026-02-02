# impl_guide_004_issueE - engine→executor 転送失敗のログ化

## Goal
- engine→executor のイベント転送失敗を明示的にログ化する

## Files（追加/変更）
- MODIFY: `crates/bf_app/src/main.rs`（仮の転送導線にログ追加）
- (将来) engine 実装箇所に統合

## Checklist
- [ ] 転送失敗がログに出る
- [ ] 失敗時に再送/停止の方針を決められる

