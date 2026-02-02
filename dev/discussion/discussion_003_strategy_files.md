# 003 - 戦略ファイル（Intent生成）方針の確定と実装計画

## 目的
戦略と実行の結合事故を防ぎつつ、将来の複数戦略同時稼働に拡張できる「戦略ファイル/戦略クレート」の方針を確定し、実装者が迷わない形で task に落とす。

## 前提（PM回答）
- 戦略は現状単一だが、将来的に複数同時にするため拡張可能な形で最初から設計する
- 戦略ロジックは当面 Event ベースのみ（タイマーは不要）
- 戦略切替は `config/run/*.toml` から include する方式で良い

## 決めたいこと（合意事項）
### A. 戦略の責務境界
- 戦略は Intent のみ生成する（`OrderRequest` を直接作らない）
- 戦略は REST/WS/DB に直接触れない（adapter を依存しない）
- 取引所仕様（tick/step/min/max、許容TIF等）は Executor に閉じ込める

### B. 複数戦略同時稼働の拡張点
最低限の拡張性として、以下を固定する。
- Strategy は `strategy_id` を持つ（ログ・trace・メトリクスのキー）
- Strategy の出力 Intent に `intent_id` と `tags`（任意）を持つ
- orchestration は `Vec<Box<dyn Strategy>>` を受け取れる形にする（将来）

### C. 設定（config）置き場と差し替え運用
推奨:
- 戦略設定は `config/strategies/*.toml`
- `config/run/*.toml` に `include.strategy = "../strategies/<name>.toml"` を追加
- 将来の複数戦略向けに `include.strategies = ["..."]` を許す余地を残す（Phase 2）

## 実装で追加したい最小要素（提案）
### 1) Intent 型（bf_core）
- `crates/bf_core/src/intent.rs` を追加
- `Intent` は strategy 側の意図を表すだけ（market/side/price_spec/size_spec/tif/flags/cancel_after_ms/tags）

### 2) Strategy trait（bf_core）
- `Strategy` は `on_event` で `Vec<Intent>` を返す
- Event ベースのみ（timer なし）

### 3) Strategy 設定のロード（bf_config）
- `config/run/*.toml` から strategy 設定を解決して DI 可能にする
- ただし「strategy のロジック」と「設定の読み込み」を混ぜない

## リスクとガード
- 戦略が `OrderRequest` を作り始めると market 制約が漏れる
- 複数戦略同時稼働時に client_order_id が衝突しやすい
  - ガード: Executor で `client_order_id` 生成規約を統一し、strategy_id/intent_id を混ぜる

## 次アクション（同意後）
- `impl_guide_002_issueD` の Intent セクションと整合させつつ、`dev/impl/impl_guide_002_issueH_strategy_skeleton.md` を作る
- 戦略クレート `crates/strategies/`（または `crates/bf_strategies/`）の雛形と、最小の Strategy 実装を追加

