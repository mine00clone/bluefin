# 001 - Strategy 設定（差し替え運用）

このディレクトリは戦略の設定ファイルを置く場所です。

## ルール
- 戦略は Intent のみ生成し、実行/正規化は Executor が担う
- `config/run/*.toml` から `include.strategy` で差し替える
- 複数戦略は将来 `include.strategies = ["..."]` で対応予定

