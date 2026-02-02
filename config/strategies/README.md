# 001 - Strategy 設定（差し替え運用）

このディレクトリは戦略の設定ファイルを置く場所です。

## ルール
- 戦略は Intent のみ生成し、実行/正規化は Executor が担う
- `config/run/*.toml` から `include.strategy` で差し替える
- 複数戦略は将来 `include.strategies = ["..."]` で対応予定

## 設定キー例
- `strategy.id`: 戦略ID
- `strategy.market`: 対象マーケット
- `strategy.side`: BUY/SELL
- `strategy.order_type`: LIMIT/MARKET
- `strategy.price_offset_bps`: 価格オフセット（bps, 例: -300）
- `strategy.size_quantity`: 数量（文字列）
- `strategy.tif`: GTC/IOC/FOK
- `strategy.post_only`: true/false
- `strategy.reduce_only`: true/false
