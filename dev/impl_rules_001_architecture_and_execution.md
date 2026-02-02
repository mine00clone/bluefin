# 001 - 実装ルール（依存の向き/責務境界/実行確認フロー）

このドキュメントは、Bluefin bot リポジトリ（`config/`, `crates/`, `data/`, `dev/`, `docs/`, `monitoring/`, `scripts/`, `tests/`, ルート`.env）において、
実装者が迷わず、戦略と実行の結合事故が起きないように「依存の向き」と「責務境界」を固定するための実装ルールである。

狙い:
- フォールバック起因の意図しない注文（勝手な発注/取消/再試行連鎖）を防ぐ
- market 仕様差（tick/step/min/max 等）を戦略から隔離する
- WS を正（source of truth）として ACK と確定を混同しない
- 設定差し替え運用（コード変更なし）を成立させる

## 1. セキュリティ & 運用（必須）
- 秘密情報は `.env` のみ（ローカル限定）。Git 管理禁止。
- ログ共有時は order_id / account 識別子 / client_id / wallet 等を必ずマスク（redact）する。
- private endpoint の raw 応答は必ず `data/raw/` に保存し、コミットしない。

## 2. SRP と境界（必須）
- Single Responsibility Principle: 1モジュール/1関数は 1責務。
- No Hardcoding: URL、銘柄、数量、TIF、フラグ、リトライ回数、タイムアウト等をコードに埋めない。必ず config 注入。
- Business-agnostic: 戦略や運用フローに依存した命名を避ける。
- Explicit Boundaries: core logic / adapters(API/DB) / orchestration(統合) を trait で境界化し、上位→下位の一方向依存に固定する。

## 3. API 実装の絶対ルール（必須）
- パース/型定義/マッピングの前に、必ず raw 応答を 1回取得して保存し、実物を目視確認する。
- 取得スクリプトは「取得→保存」までで止め、憶測でデコード実装に入らない。

## 4. レイヤ定義と依存方向（固定）

### Layer A: Foundation（純粋・I/Oなし）
- 対象（現状）: `crates/bf_core`
- ルール: A は他レイヤに依存しない。

### Layer B: Adapters（外部 I/O）
- 対象（現状）: `crates/bf_rest`, `crates/bf_ws`, `crates/bf_auth`, `crates/bf_storage_sqlite`
- ルール:
  - B は A に依存してよい。
  - ただし B は業務判断（発注可否/リスク/自動フォールバック）はしない。薄いラップと raw 保存まで。

### Layer C: Domain Services（状態/整合/支援）
- 対象（現状）: `crates/bf_order_manager`（OMS）, `crates/bf_balance`, `crates/bf_history`
- ルール:
  - C は状態管理/整合（snapshot + WS + reconcile）を担う。
  - C は勝手に発注しない（実行は D のみ）。

### Layer D: Application（戦略/実行/統合）
- 対象（現状）: `crates/bf_order_exec`, `crates/bf_app`
- ルール:
  - 戦略は「意図（Intent）」生成のみ。
  - 実行は D に集約し、REST/WS/DB を戦略が直接触らない。

## 5. 「戦略」と「実行」の分離ルール
- 戦略は REST 直結の型（例: `bf_core::OrderRequest` 相当）を直接作らない。
- 戦略の出力は Intent/Plan（例: offset_bps, quantity, tif, post_only, reduce_only, cancel_after_ms 等の“意図”）とする。
- Executor（発注窓口）が market 制約と API 制約を一元吸収する。

## 6. Market メタ（tick/step/min/max 等）の取り扱い
- 推奨: ローカル snapshot を正とする。
  - `config/markets/` の snapshot を読み、実行時の network 依存を最小化する。
- 実装責務:
  - snapshot ロード/供給は config 層（例: `bf_config`）が行い、Executor は供給されたメタを使って正規化する。

## 7. WS を正にする確認フロー（固定）
- REST 応答 = 受付/ACK（成功扱いしない）。
- WS account stream = 状態確定（open/fill/cancel 等）。
- Executor は次の順序を固定する:
  1) Intent -> Normalize（MarketMeta）
  2) Risk Gate（kill switch / 上限 / 注文件数等）
  3) Idempotency（client_order_id を強制・生成）
  4) REST Send
  5) Confirm（WS account で確定）
  6) Timeout fallback（必要なら openOrders 等で補助確認）
  7) Persist（SQLite + raw）

## 8. フォールバック/エラー処理の安全原則（重要）
- 未知値/未指定値を黙って既定値に落とさない（例: unknown TIF -> GTC などは禁止）。
- 「失敗を Ok で返す」設計は上位が成功扱いする温床になるため禁止。失敗は `Err` で伝播する。
- cancel-all 等の曖昧な返却（空配列など）を「成功」と誤認できない API/型にする。
- market snapshot が空/欠損のときに検証をスキップしない。安全側停止（実行ブロック）を優先する。
- leverage 等のパラメータをハードコードしない。設定注入で明示する。

## 9. 設定（config）差し替え運用
- 実行入口は `config/run/*.toml` とし、参照先（profile/plan/markets）を差し替える。
- live 実行には明示的な safety gate（例: `ALLOW_LIVE_TRADING=YES`）を必須とする。

