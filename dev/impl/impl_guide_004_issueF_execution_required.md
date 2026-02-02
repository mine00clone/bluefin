# impl_guide_004_issueF - run config 経由時の [execution] 必須化

## Goal
- run config 経由で起動する場合、[execution] が欠落していたら Err にする

## Files（追加/変更）
- MODIFY: `crates/bf_config/src/lib.rs`
- MODIFY: `config/app.toml`（必須項目の明示）
- MODIFY: `config/run/README.md`（実行方針の追記）

## Checklist
- [ ] [execution] 欠落で起動時に Err
- [ ] README に方針が明記される

