# impl_guide_002 - Config/Plan/MarketMeta を核にした戦略運用基盤（Rust）

## Goals
- 複数銘柄・複数注文・複数戦略を「設定差し替え」で安全に運用できる
- Executor が market制約（tick/step/min/max）を一元吸収し、戦略を汚さない
- WS（正） + REST（補助/実行）で、ACK と確定を分けて扱える
- API実装は必ず raw を保存→目視→解析（例外なし）

## Non-Goals（Phase 1ではやらない）
- Strategy DSL（条件ファイルによる戦略記述）
- 高度なテンプレエンジン（plan の変数/ループ等）
- ホットリロード（将来検討）

## High-level Policy
- Single Responsibility / No Hardcoding / Config Injection / Explicit Boundaries を厳守
- config は run.toml を入口に「参照ファイルを切り替える」方式
- market snapshot は実行時通信を避ける（Phase 1）
- Strategy は Intent を生成し、Plan+Risk がゲートし、Executor が正規化/送信する

## Phase Plan
### Phase 1（最短で運用摩擦を下げる）
1. config/run + profiles + plans + markets snapshot を導入
2. market snapshot を参照して正規化（実行時 market fetch を禁止）
3. create/cancel の raw 取得と、最小の execution パイプラインを整備

### Phase 2（運用の安全性を上げる）
1. universe / strategies / risk を分離・拡張
2. OMS/Balance/History と統合して「状態に基づくリスクゲート」を強化
3. 監視・アラート・再現性（raw/DB）を拡充

## Verification Methods（DoD）
- config/run の差し替えだけで env/plan が切り替わる（コード修正不要）
- 同一 plan を複数銘柄に適用できる
- tick/step/min/max に合わない注文は送信前に fail-fast する（または snap し、レポートが残る）
- API呼び出しは必ず raw 保存（data/raw/）される
- Cancel は REST ACK と WS 確定を分けて扱う（ACKだけで成功扱いしない）

## Task Breakdown（親）
- Issue A: Config 分割 + Run Loader（bf_config）
- Issue B: Market Snapshot パイプライン（raw→生成）+ MarketMetaStore（bf_core/bf_rest）
- Issue C: Plan モデル（複数注文）+ バリデーション（bf_core/bf_config）
- Issue D: Intent → Gate(Plan/Risk) → Normalize → Send（bf_order_exec など）
- Issue E: examples/scripts の更新（market実行時fetch撤廃、raw保存徹底）
- Issue F: テスト/fixtures/ドキュメント整備
- Issue H: Strategy スケルトン（Intent生成のみ）+ 設定差し替え導線
