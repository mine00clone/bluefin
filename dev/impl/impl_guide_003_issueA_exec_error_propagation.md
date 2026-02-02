# impl_guide_003_issueA - Exec の失敗伝播（Ok に包まない）と上位停止

## Goal
- `create_order` / `cancel` の「実行失敗」を `Ok(...)` に包まず、`Err` として上位に伝播する
- 呼び出し側が `Result` だけ見ても誤って成功扱いできない状態にする
- ACK（受付）と確定（WS）分離を保ったまま、ACK 失敗は必ず停止させる

## Background / Risk
- 現状は executor のエラーを `Ok(Rejected/Failed)` に変換しており、上位が成功扱いすると意図しない連鎖動作が起き得る
  - 該当: `crates/bf_order_exec/src/lib.rs:66` `crates/bf_order_exec/src/lib.rs:102`

## Scope
- OrderExecService の public API と戻り値セマンティクスを整理
- 呼び出し側が「ACK 成功」か「実行失敗」かを型で区別できるようにする

## Files（追加/変更）
- MODIFY: `crates/bf_order_exec/src/lib.rs`
- (必要なら) MODIFY: `crates/bf_core/src/traits.rs`（上位が使う統一 interface の戻り値を揃える場合）
- (必要なら) MODIFY: `crates/bf_order_exec/src/bluefin_executor.rs`（エラー型の整流化）

## Proposed API / Types
- `OrderExecService::create_order`:
  - Before: `Result<ExecResult, ExecError>`（executor error を `Ok(Rejected)` に包む）
  - After: `Result<ExecAck, ExecError>`（executor error は `Err(ExecError::CreateFailed(..))`）
- `OrderExecService::cancel`:
  - Before: `Result<CancelResult, ExecError>`（executor error を `Ok(Failed)` に包む）
  - After: `Result<CancelAck, ExecError>`（executor error は `Err(ExecError::CancelFailed(..))`）

`ExecAck` / `CancelAck` は「REST 受付済み」を表すだけに限定し、確定は WS 側に委譲する。

## Behavior Changes
- executor 失敗時（署名失敗、HTTP 失敗、API エラー等）に `Err` が返る
- `Ok` は「受付（ACK）した」ケースのみ
- これにより上位の制御フロー（次の注文、状態更新、リトライ等）は `?` で自然に停止できる

## Checklist
- [x] `OrderExecService` の public API が「失敗を Ok に包まない」ことを型で保証する
- [x] ログが「成功/失敗」を誤解させない（Rejected/Failed を success として出さない）
- [x] バッチ作成（create_orders）が 1件失敗を握り潰さず、呼び出し側が扱える
- [x] 既存の examples / scripts が新しい戻り値に追従できる

## Verification
- `cargo test -p bf_order_exec`
- 「executor がエラーを返すモック」を差し込み、`Err` 伝播を確認
