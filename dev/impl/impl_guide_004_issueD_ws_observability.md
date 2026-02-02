# impl_guide_004_issueD - WS 欠落/パース失敗の可観測化

## Goal
- WS データ欠落・パース失敗を Raw + Error で可観測化する

## Files（追加/変更）
- MODIFY: `crates/bf_ws/src/lib.rs`
- MODIFY: `crates/bf_ws/examples/*`（raw 保存の一貫性）

## Checklist
- [x] parse 失敗時に raw と error が保存される
- [x] 欠落・異常イベントがログに残る
