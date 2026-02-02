# impl_guide_003_issueG - leverage ハードコード排除（設定注入）

## Goal
- `BluefinOrderExecutor::new` の leverage 10x ハードコードを撤廃し、設定注入に置き換える
- 設定欠落時の暗黙フォールバックで過剰レバレッジにならないようにする

## Background / Risk
- 現状は 10x 固定（e9 で設定）になっている
  - 該当: `crates/bf_order_exec/src/bluefin_executor.rs:37`

## Scope
- leverage の供給元を config に移し、Executor 生成時に注入する

## Files（追加/変更）
- MODIFY: `crates/bf_order_exec/src/bluefin_executor.rs`
- MODIFY: `crates/bf_config/src/lib.rs`（RuntimeConfig のどこに置くか）
- MODIFY: `config/app.toml`（または `config/profiles/*.toml` / `config/plans/*.toml`）
- (将来) MODIFY: `crates/bf_app/src/main.rs`（DI で executor を生成する箇所が出来たら注入）

## Proposed Config Placement
- 案1（推奨）: `app.execution.default_leverage` を追加（RunConfig の合成に乗せる）
- 案2: plan 側に `leverage` を持たせる（注文単位で指定）

デフォルトは「安全側」(例: 1x) とし、live は明示値必須にする案も検討する。

## Checklist
- [ ] leverage がコードに埋まっていない
- [ ] config 欠落時は fail-fast か安全側（明示した方針）になる
- [ ] 実行ログに leverage の実効値が記録される（監査性）

## Verification
- `cargo test -p bf_order_exec`（executor の leverage が設定で変わる）
