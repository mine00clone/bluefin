# 004 - Exec/Confirm パイプラインと運用系ガードの必須化

## 目的
Normalize→Send→Confirm を本流として固定し、WS 確定と fallback の扱いを明示化する。
また、WS 欠落/パース失敗/転送失敗などの運用上の事故ポイントを可観測化する。

## 合意事項（PM回答）
- A項目はすべて実施する
- CLI は raw 出力中心で最小機能から始める

## 対象（A: 必須）
1) Normalize→Send→Confirm パイプラインの明確化と実装
2) Confirm ステータスの詳細化（Active/Filled/PartiallyFilled/Canceled/Expired/TimedOut）
3) fallback は opt-in（`fallback_enabled`）
4) WS data 欠落・パース失敗を Raw + Error で可観測化
5) engine→executor event 転送失敗のログ化
6) run config 経由時に [execution] 必須（欠落で Err）

## 非対象（今回やらない）
- 高度な DSL / ホットリロード
- CLI の高機能化（raw 出力優先）

## 期待する成果
- WS 確定を前提に注文状態が遷移する
- fallback は明示 opt-in でしか発動しない
- WS の不具合や engine/executor の転送失敗がログで特定可能
- run config に execution が無い場合に即エラー（設定の暗黙補完を排除）

