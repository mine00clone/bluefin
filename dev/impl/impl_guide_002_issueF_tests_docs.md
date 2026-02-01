# impl_guide_002_issueF - tests/fixtures と docs の整備

## Goal
- raw と fixture を分離し、再現性と秘匿性を両立する
- OMS/正規化/plan validation を自動テストで守る

## Files（追加/変更）
- ADD: tests/fixtures/rest/*.json（マスク済み）
- ADD: tests/fixtures/ws/*.ndjson（マスク済み）
- ADD: crates/*/tests/*.rs
- MODIFY: dev/planning/planning_001.md（Phase追記）
- MODIFY: dev/reference_*.md（market snapshot 更新手順リンク）

## Checklist
- [ ] raw（data/raw）と fixture（tests/fixtures）が分離されている
- [ ] plan validation / normalization が unit test で検証できる
- [ ] doc が “どう更新するか/どう切り替えるか” を説明できる
