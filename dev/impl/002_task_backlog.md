# 002 - Next Functional Improvements Backlog (Post impl_guide_002 A/B/C)

目的: 「戦略として柔軟に運用できる注文基盤」に向けて、次の本質改善を優先度順に整理する。

## P0: 最優先（運用事故を減らす / 戦略運用の前提）
- [ ] Issue D-1: MarketMetaStore + 正規化（Executorに一元化）
  - `crates/bf_core/src/traits.rs`: `trait MarketMetaStore` を追加（snapshot/将来DBの差し替え点）
  - `crates/bf_core/src/normalization.rs`: price/size の snap|reject、rounding、deviation を実装（e9整数）
  - `crates/bf_order_exec/`: create/cancel 前に必ず正規化・fail-fast（例: tick/step/min/max）
  - DoD: config の差し替えだけで snap/reject 等が切り替わる（ビルド不要）

- [ ] Issue D-2: Plan/Intent からの実行経路を確立（example依存をやめる）
  - `crates/bf_core/src/intent.rs`: Strategyが吐く最小 Intent を定義（注文ではなく意図）
  - `crates/bf_app/`: `--run` で起動し、plan/market snapshot/normalization を通して dryrun/paper/live を切替
  - DoD: `create_order_raw.rs` 相当の動作を「アプリ経由」で再現できる

## P1: 次点（正しさ/再現性/可観測性）
- [ ] Issue D-3: ACK と WS 確定の分離を実装として固定化
  - `crates/bf_order_manager/`: WSイベントで状態遷移、REST(openOrders)でreconcile
  - Cancel: ACK成功≠成功扱い禁止（WS確定で成功/失敗を決める）

- [ ] Issue F-1: 正規化/plan validation の unit test 化（fixturesベース）
  - `tests/fixtures/`: raw と分離したマスク済み fixture を追加
  - `crates/bf_core/tests/`: normalization の丸め・min/max・deviation のテスト
  - `crates/bf_config/tests/`: run/plan/snapshot のバリデーションテスト

## P2: 将来（運用の拡張性）
- [ ] 複数戦略/複数ユニバース/複数planの同時運用（run の合成）
- [ ] market snapshot の更新自動化（起動時fetch/SQLite化はPhase2以降で再検討）
- [ ] ホットリロード（Phase2以降）

## Notes
- `.env` は secrets のみに限定し、運用パラメータは必ず `config/` 注入。
- raw 保存は継続（REST/WS）。ログは原則 redaction し、詳細は raw/fixture を参照する。
