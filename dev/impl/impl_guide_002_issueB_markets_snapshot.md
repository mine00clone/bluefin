# impl_guide_002_issueB - market snapshot（raw→目視→生成）+ MarketMetaStore

## Goal
- 実行時に market info を毎回取りに行かず、ローカル snapshot を参照して正規化できる
- API関連ルールに従い、必ず raw を保存してから設計・パースを行う

## Files（追加/変更）
- ADD: crates/bf_rest/examples/raw_exchange_info_dump.rs
- ADD: scripts/fetch_exchange_info_raw.sh（任意）
- ADD: data/raw/rest/（gitignore。実行で生成）
- ADD: crates/bf_core/src/market_meta.rs（または相当ファイル）
- ADD: crates/bf_core/src/traits/market_meta_store.rs
- ADD: crates/bf_market_meta_snapshot（または bf_storage_sqlite 内でも可）
- ADD: config/markets/snapshot.json（生成物）
- ADD: config/markets/README.md（更新手順）
- MODIFY: crates/bf_order_exec（正規化で利用）
- MODIFY: crates/bf_rest/examples/create_order_raw.rs（実行時fetch禁止へ）

## Raw取得（必須）
- raw_exchange_info_dump.rs の挙動:
  - /exchange/info を 1回呼ぶ
  - レスポンス全文を data/raw/rest/exchange_info_YYYYMMDD_HHMMSS.json に保存
  - そのファイルパスをログに出す（redactionあり）

## Snapshot 生成（注意）
- ルール: raw を目視確認するまで、構造化/モデル化しない
- 目視確認後に SnapshotSchema を確定し、生成ツールを作る（別scriptでもOK）
  - 入力: data/raw/rest/exchange_info_*.json
  - 出力: config/markets/snapshot.json

## Structures（bf_core）
- struct MarketId(String)
- struct MarketMeta
  - tick_size_e9: i64
  - step_size_e9: i64
  - min_order_quantity_e9: i64
  - (必要なら) min/max price/qty/notional 等
- trait MarketMetaStore
  - fn get(&self, market: &MarketId) -> Result<MarketMeta>
  - fn list(&self) -> Vec<MarketId>

## Notes
- tickSizeE9/stepSizeE9/minOrderQuantityE9 が /exchange/info に存在する想定（ただし実装は raw で検証してから確定）

## Checklist
- [x] raw を保存し、目視確認できる（raw dump example + script 追加）
- [x] snapshot を生成し、実行時通信なしで読み込める（generator tool 追加）
- [ ] Executor が tick/step/min を使って fail-fast or snap できる
- [ ] examples が「実行時に exchange/info を取らない」状態になった
