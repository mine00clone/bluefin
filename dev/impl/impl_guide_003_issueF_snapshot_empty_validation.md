# impl_guide_003_issueF - market snapshot 空の検証スキップ撤廃（安全側停止）

## Goal
- market snapshot が空/欠損のときに plan 検証をスキップしない
- 未知 market の plan が通って注文作成に進むリスクを排除する

## Background / Risk
- `validate_plan_against_snapshot` が `snapshot.markets.is_empty()` の場合 `Ok(())` でスキップする
  - 該当: `crates/bf_config/src/lib.rs:262`

## Scope
- snapshot 空の場合はデフォルトでエラーにする（fail-fast）
- テストや開発用にどうしても空を許容する場合は、明示的な config で opt-in にする

## Files（追加/変更）
- MODIFY: `crates/bf_config/src/lib.rs`
- (必要なら) MODIFY: `config/run/*.toml`（許容フラグを設ける場合）

## Proposed Config
- 例: `run.mode.allow_empty_markets_snapshot = false`（デフォルト false）

## Checklist
- [ ] snapshot 空なら plan 実行が停止する（デフォルト）
- [ ] 例外許容は明示的な config opt-in でのみ可能
- [ ] エラーメッセージが運用で原因特定しやすい（どの snapshot が空か）

## Verification
- `cargo test -p bf_config`（空 snapshot を与えたときに Err になる）
