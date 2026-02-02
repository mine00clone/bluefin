# impl_guide_002_issueC - Plan（複数注文）モデル + 設定スキーマ

## Goal
- 単発注文を config に固定せず、複数銘柄・複数注文を plan として表現できる
- float禁止（精度事故防止）、設定差し替えで運用できる

## Files（追加/変更）
- ADD: config/plans/plan_live.toml
- ADD: config/plans/plan_paper.toml
- ADD: crates/bf_core/src/plan.rs
- ADD: crates/bf_config/src/plan_config.rs
- MODIFY: crates/bf_config/src/lib.rs（RuntimeConfigに plan を統合）

## PlanConfig（TOML案：Phase1最小）
- [[orders]] の配列で複数注文を表現
- price/size は string or integer（float禁止）
- 価格は absolute or offset_bps（参照価格は WS の mid/mark/last 等を後で選べるようにフィールドだけ置く）

例:
[[orders]]
id = "btc_entry_1"
market = "BTC-PERP"
side = "buy"
order_type = "limit"
price_mode = "offset_bps"
price_bps = "-30"          # string
size_mode = "quantity"
quantity = "1000000"       # e9整数を string（Phase1はこれでOK）
tif = "GTC"
post_only = true
reduce_only = false
cancel_after_ms = 15000

## Structures（bf_core）
- enum Side { Buy, Sell }
- enum OrderType { Limit, Market }
- enum TimeInForce { GTC, IOC, FOK }
- struct OrderFlags { post_only: bool, reduce_only: bool, ... }
- enum PriceSpec
  - AbsoluteE9(i64)
  - OffsetBps { bps: DecimalString, reference: RefPriceKind }
- enum SizeSpec
  - QuantityE9(i64)
  - (Phase2) Notional / BalanceRatio etc
- struct PlanOrder
  - id: String
  - market: MarketId
  - side: Side
  - order_type: OrderType
  - price: PriceSpec
  - size: SizeSpec
  - tif: TimeInForce
  - flags: OrderFlags
  - cancel_after: Option<Duration>
- struct Plan { orders: Vec<PlanOrder> }

## Validation（bf_config or bf_core）
- Live の場合: post_only のデフォルトなどを明確化
- 参照市場が snapshot に存在しない場合は fail-fast（起動時に弾く）
- order数上限（後で risk へ移す）

## Checklist
- [x] plan_* の差し替えだけで「複数注文」が切り替わる（run.toml で plan 切替対応）
- [x] float無しで表現できる（string/整数のみ）
- [x] 起動時に plan の整合性チェックが走る（snapshot照合 + モード検証）
