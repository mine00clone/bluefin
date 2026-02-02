# impl_guide_003_issueB - バリデーション迂回防止（価格 0 フォールバック封止）

## Goal
- `OrderExecutor` を直接呼ばれても危険な注文（Limit 価格なし/価格 0 等）が作れないようにする
- `build_order_request` の `price_e9="0"` フォールバックを「Market のみに限定」し、Limit では必ず停止する

## Background / Risk
- `build_order_request` が `request.price=None` の場合に `price_e9="0"` を作る
  - 該当: `crates/bf_order_exec/src/bluefin_executor.rs:134`
- `OrderExecService::validate_order` を通らない経路（直に `OrderExecutor` を使う等）で事故り得る

## Scope
- Executor 実装側（BluefinOrderExecutor）にも最低限の fail-fast validation を持たせる

## Files（追加/変更）
- MODIFY: `crates/bf_order_exec/src/bluefin_executor.rs`
- (必要なら) MODIFY: `crates/bf_core/src/order.rs`（OrderRequest の制約を型で表現したくなった場合）

## Implementation Notes
- `build_order_request` 内で以下を検証して `CoreError::InvalidRequest` を返す
  - `request.size > 0`
  - `request.order_type == Limit` の場合 `request.price.is_some()`
  - `request.order_type == Market` の場合 `price_e9` は `0` を許可（ただし Limit では禁止）
- 「post_only + 非GTC」は warn ではなく拒否（or strict mode により拒否）を検討

## Checklist
- [ ] Limit で `price=None` を渡すと executor 内で必ず `Err` になる
- [ ] Market の `price=None` は許可し、`price_e9=0` が意図通りに限定されている
- [ ] post-only と TIF の組み合わせが危険な場合に fail-fast できる
- [ ] 既存の `OrderExecService::validate_order` と二重化しても矛盾しない

## Verification
- `cargo test -p bf_order_exec` に validation 迂回ケースのテストを追加
