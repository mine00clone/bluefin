# 003 - フォールバック/エラー処理強化による意図しない注文防止

## Goals
- フォールバックや黙示的デフォルトが原因で、勝手に注文が実行されるリスクを排除する
- 失敗時は fail-fast し、上位にエラーを明示的に伝播する
- 仕様外/未知入力は「安全側停止」を徹底する
- ルール（Single Responsibility / No Hardcoding / Config Injection / Explicit Boundaries）を守る

## Non-Goals
- 新しい戦略ロジックの追加
- 成約アルゴリズムや価格決定ロジックの刷新
- 取引所SDKの置き換え

## Key Points / Precautions
- ACK と確定は必ず分離し、ACK失敗を成功扱いしない
- 既定値への黙示的フォールバックは「安全側停止」に置換する
- 不正な状態遷移や不整合は warn で流さず、操作を止める導線を設ける
- market snapshot 空のときは安全側に倒し、plan 実行をブロックする
- ハードコードされた実行パラメータ（例: leverage）は設定注入に置換する

## Implementation Policy
- エラーは `Err` として伝播し、上位で明示的に扱う
- 不明値/未指定値は「拒否」または「明示的な設定がある場合のみ許可」
- 取り扱いが曖昧なフォールバックは config に明示化し、デフォルトは安全側
- ログは監査目的で必要最小限に整理（肥大化抑止）

## Verification Methods
- create/cancel 失敗時に `Result::Err` が上位に伝播し、以降の処理が停止する
- Limit 価格なし / 未知の TIF/OrderType/Side の場合に注文が送信されない
- cancel-all で返却結果が空配列のときでも「成功」と誤解されない
- market snapshot が空の場合は plan 実行が停止する
- leverage 等の実行パラメータが設定ファイル経由で注入される

## Task Breakdown
### Issue A: OrderExec の失敗伝播とガード強化
- `OrderExecService::create_order` / `cancel` の失敗を `Err` で返す方針に変更
- `OrderExecService::validate_order` を「必ず通す」実行経路に統一
- ACK/Rejected の取り扱いを上位と一貫させる

### Issue B: SDK 変換のフォールバック撤廃
- `OrderSide::Unspecified` / 未知の `OrderType` / 未知の `TimeInForce` を拒否
- `price_e9 = 0` フォールバックを廃止し、Limit で price 不在は必ず停止
- `post_only`/`tif` の不整合は失敗として扱う方針を検討

### Issue C: cancel-all の曖昧結果を解消
- cancel-all の返却仕様を明確化し、空配列での誤判定を防止
- 上位の取り扱い方針（成功/失敗/不明）を決める

### Issue D: OMS/WS の不正遷移ハンドリング
- `apply_update` の不正遷移での「継続」方針を見直し
- 不整合検出時の安全側停止（取引停止 or 再同期）を設計

### Issue E: Config 安全ガード
- market snapshot 空時の検証スキップを廃止し、実行ブロック
- leverage など実行パラメータのハードコード排除（config へ移管）
- ランタイム config の安全検証を拡充

## Reference Findings (要約)
- 失敗を `Ok(Rejected/Failed)` で返すことで上位が成功扱いし得る
- SDK 変換で未知値が黙って既定値に落ちる
- cancel-all の返却が空配列で誤解され得る
- 不正遷移が warn のみで継続される
- market snapshot が空でも plan が通る
- leverage がハードコードされている
