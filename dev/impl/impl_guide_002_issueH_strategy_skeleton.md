# impl_guide_002_issueH - 戦略スケルトン（Intent 生成）と設定差し替え導線

## Goal
- 戦略は Intent のみ生成し、実行（Normalize/Gate/Send/Confirm）と分離する
- Event ベースで動作する Strategy trait を用意する（timer は扱わない）
- 将来の複数戦略同時稼働に拡張できる土台（strategy_id/intent_id/tags）を持つ
- 戦略設定は `config/run/*.toml` から include で差し替え可能にする

## Non-Goals
- Strategy DSL の導入
- ホットリロード
- 高度なスケジューリング（timer/cron）

## Design (Rules)
- 戦略は `bf_rest`/`bf_ws`/`bf_storage_sqlite` に依存しない
- 戦略は `OrderRequest` を作らない（Intent のみ）
- market 制約・API 制約は Executor に閉じ込める

## Files（追加/変更）
- ADD: `crates/bf_core/src/intent.rs`
- MODIFY: `crates/bf_core/src/lib.rs`（intent を export）
- (Option) ADD: `crates/strategies/Cargo.toml`
- (Option) ADD: `crates/strategies/src/lib.rs`
- MODIFY: `crates/bf_config/src/lib.rs`（RunInclude に strategy を追加する場合）
- MODIFY: `config/run/*.toml`（include.strategy を追加する場合）
- ADD: `config/strategies/README.md`
- ADD: `config/strategies/example.toml`

## Interfaces
- `bf_core::Strategy`
  - `fn strategy_id(&self) -> &str`
  - `fn on_event(&mut self, ctx: &StrategyContext, event: MarketEvent) -> Vec<Intent>`
- `bf_core::Intent`
  - `intent_id: String`
  - `strategy_id: String`
  - `market: String`
  - `side: Side`（または String で後段変換）
  - `price_spec`（absolute/offset_bps 等）
  - `size_spec`（quantity 等）
  - `tif` / `post_only` / `reduce_only` / `cancel_after_ms`
  - `tags: Vec<String>`

## Checklist
- [ ] Strategy が Intent のみを生成する（OrderRequest を触らない）
- [ ] 将来の複数 Strategy を `Vec<Box<dyn Strategy>>` で扱える
- [ ] 戦略設定が `config/run/*.toml` の include で差し替え可能
- [ ] executor 側で client_order_id を strategy_id/intent_id 由来で一意化できる余地がある

## Verification
- `cargo test`（compile が通る）
- (Optional) ダミー MarketEvent を与えて Intent が生成される unit test

