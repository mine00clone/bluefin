# impl_guide_004 - Exec/Confirm パイプライン強化と可観測性の必須化

## Goals
- Normalize→Send→Confirm を標準パイプラインとして固定する
- Confirm を詳細ステータス化し、WS確定を前提に扱う
- fallback は opt-in（`fallback_enabled`）でのみ許可
- WS 欠落/パース失敗/転送失敗を Raw+Error で可観測化
- run config 経由時に [execution] 必須（欠落で Err）

## Non-Goals
- 高度な DSL/ホットリロード
- CLI の高機能化（raw 出力優先）

## Implementation Policy
- WS を正とし、REST ACK は補助
- fallback は明示設定のみ
- 失敗は Err で伝播、曖昧フォールバック禁止

## Verification Methods
- Confirm の詳細ステータスが一意に判定できる
- fallback は `fallback_enabled=true` 時のみ発動
- WS 欠落/パース失敗/転送失敗がログに残る
- run config に [execution] が無い場合は起動時に Err

## Task Breakdown
- Issue A: Confirm ステータス詳細化（Active/Filled/PartiallyFilled/Canceled/Expired/TimedOut）
- Issue B: Normalize→Send→Confirm の実装統一（Executor 側）
- Issue C: fallback opt-in の導入（execution.fallback_enabled）
- Issue D: WS 欠落/パース失敗の Raw+Error 可観測化
- Issue E: engine→executor 転送失敗のログ化
- Issue F: run config 経由時の [execution] 必須化
