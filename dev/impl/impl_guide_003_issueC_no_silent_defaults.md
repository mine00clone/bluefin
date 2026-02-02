# impl_guide_003_issueC - 未知値の黙示フォールバック撤廃（Side/Type/TIF）

## Goal
- 未知/未指定の値を黙って安全でない既定値に落とさない
- 不明な値は「拒否」し、WS/REST の仕様差や SDK 生成差異を raw capture に戻せるようにする

## Background / Risk
- 現状、複数箇所で黙って既定値に落としている
  - `OrderSide::Unspecified -> Sell` `crates/bf_order_exec/src/bluefin_executor.rs:198`
  - `OrderType::_ -> Limit` `crates/bf_order_exec/src/bluefin_executor.rs:202`
  - `OrderTimeInForce::_ -> Gtc` / `None -> Gtc` `crates/bf_order_exec/src/bluefin_executor.rs:211`
- 内部状態が誤ると、後続の再注文/ヘッジ判断が逆方向に進む可能性がある

## Scope
- BluefinOrderExecutor の「SDK -> bf_core 型変換」を strict 化
- 将来 WS/REST decode でも同様の方針（unknown -> Err）を適用できる土台を作る

## Files（追加/変更）
- MODIFY: `crates/bf_order_exec/src/bluefin_executor.rs`
- (必要なら) ADD: `crates/bf_core/src/strict.rs`（unknown を弾く共通 helper）

## Implementation Notes
- `execute_create_order` の Order 構築時に unknown variant を見つけたら `Err(CoreError::InvalidRequest(..))`
  - `signed_fields.side` が `Unspecified` なら Err（この値が出た場合は upstream のバグとして扱う）
  - `signed_request.r#type` / `signed_request.time_in_force` の unknown も Err
- 例外的に「欠損を許す」場合は、許可条件を config で明示し、デフォルトは拒否

## Checklist
- [x] unknown/unspecified を既定値に落とさず、必ず Err で停止する
- [x] Err メッセージに raw capture の参照情報（どのフィールドが unknown か）が含まれる
- [x] ログが過剰に肥大化しない（必要最小限の info/error）

## Verification
- `cargo test -p bf_order_exec`
- unknown variant の擬似投入テストは別途検討
