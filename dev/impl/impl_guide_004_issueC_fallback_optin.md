# impl_guide_004_issueC - fallback opt-in（execution.fallback_enabled）

## Goal
- fallback は明示 opt-in のみで有効化する
- 既定は無効（false）

## Files（追加/変更）
- MODIFY: `config/app.toml`
- MODIFY: `crates/bf_config/src/lib.rs`
- MODIFY: `crates/bf_order_exec`（fallback 使用箇所）

## Checklist
- [ ] fallback_enabled が config に追加される
- [ ] false の場合は一切 fallback しない
- [ ] true の場合のみ openOrders 等を使う

