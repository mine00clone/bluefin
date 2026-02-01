# impl_guide_002_issueD - Intent → Gate(Plan/Risk) → Normalize → Send → Confirm

## Goal
- Strategy は Intent を出すだけ
- Plan と Risk で「許可/拒否」を行い
- Executor が marketメタで正規化し、REST送信し、WSで確定を待つ

## Files（追加/変更）
- ADD: crates/bf_core/src/intent.rs
- ADD: crates/bf_core/src/gates/{plan_gate.rs,risk_gate.rs}
- MODIFY: crates/bf_order_exec/src/lib.rs
- MODIFY: crates/bf_ws（WS確定イベントの取り回し）
- MODIFY: crates/bf_order_manager（WSイベントで状態更新）
- ADD: crates/bf_core/src/normalization.rs（正規化レポート含む）

## Interfaces（bf_core）
- trait Strategy
  - fn on_event(&mut self, ctx: &StrategyContext, event: MarketEvent) -> Vec<Intent>
- struct Intent
  - intent_id, market, side, order_type, price_spec, size_spec, tif, flags, timeouts, tags
- trait PlanGate
  - fn filter(&self, intent: &Intent) -> GateDecision
- trait RiskGate
  - fn filter(&self, intent: &Intent, snapshot: &AccountSnapshot, oms: &OmsSnapshot) -> GateDecision
- trait OrderExecutor
  - fn submit(&self, req: NormalizedOrderRequest) -> AckHandle
- trait OrderConfirmer
  - fn wait_final(&self, ack: AckHandle, timeout: Duration) -> FinalOrderResult

## Normalization Policy（config注入）
- execution.normalization.price.policy = snap|reject
- execution.normalization.price.rounding = down|up|nearest
- execution.normalization.size.policy = snap|reject
- execution.normalization.size.rounding = down|up|nearest
- execution.normalization.max_deviation_bps = "5"（string）

## Checklist
- [ ] Intent と 実行詳細（署名/REST/WS確定）が分離されている
- [ ] market制約は Executor 側で一元適用される
- [ ] Cancel は ACK と WS確定を分けて扱う（ACKのみ成功扱い禁止）
- [ ] すべての API 呼び出しが raw 保存される（運用再現性）
