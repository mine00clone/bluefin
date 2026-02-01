# discussion_002 - 戦略として柔軟に運用できる「注文/設定/marketメタ」ゼロベース設計

## 目的
複数銘柄・複数注文・複数戦略を安全・高速に回すため、
- 設定の責務分離
- market制約（tick/step/min/max）の吸収
- 「Strategy(意図)」と「Execution(正規化/送信/確認)」の分離
をゼロベースで確定する。

## 背景 / 現状課題（要約）
- configが単発実行向けで、銘柄/注文を増やすほど運用事故が増える
- tickSizeE9 等の制約に合わず reject/cancel が発生
- 実行時に market info を毎回取るのは遅い/不安定
- 設定変更がコード/ビルドに依存すると運用不能

## 提案アーキテクチャ（方針）
### 設定の分離（採用候補）
- config/app.toml           : アプリ共通（db/log/path 等）
- config/profiles/*.toml    : 環境差分（URL/WS/colocated 等）
- config/markets/snapshot.json : 市場仕様（tick/step/min/max、許容注文タイプ/TIF…）
- config/universe/*.toml    : 監視銘柄群
- config/strategies/*.toml  : 戦略パラメータ
- config/plans/*.toml       : 注文計画（複数注文/注文グループ）
- config/risk/*.toml        : 上限/回数/kill switch
- config/run/*.toml         : 上記をどう組み合わせて動かすか（参照ファイル指定）

### Strategy と Execution の分離（採用候補）
- Strategy: Intent を生成（何をしたいか）
- Execution: marketメタを用いて正規化し、署名/送信/ACK/WS確定を扱う（どう実現するか）

### market制約は Executor 側で一元化（採用候補）
- tick/step/min/max は銘柄ごと
- 戦略が知ると設計が汚れるため、Executor で正規化ポリシーを統一する
- ポリシーは config 注入（snap / reject、丸め方向、許容乖離）

## market snapshot の方針（選択肢）
Option A: config/markets/snapshot.json を正（推奨）
- scripts で手動更新
- 実行時は通信せず snapshot 参照のみ

Option B: 起動時1回取得して data/ にキャッシュ
- 自動更新できるが、起動時ネットワーク依存が増える

## 意思決定が必要な項目（PM回答が必要）
D1. 分離構成（app/profile/markets/universe/strategies/plans/risk/run）をフル採用するか？
    - いきなりフル導入 or 段階導入（Phase 1 から）のどちらにするか

D2. market snapshot の正をどれにするか？
    - Option A(JSON) / Option B(起動時取得) / SQLite

D3. Plan の表現（Phase 1 の最小仕様）
    - TOML配列の複数注文（テンプレ無し）
    - 価格/数量は float禁止（string/整数）
    - price_mode / size_mode をどこまで許すか（例: absolute / offset_bps）

D4. Strategy の表現
    - Rust実装 + TOMLパラメータ（推奨）
    - DSL化は Phase 2 以降でよいか

D5. 正規化ポリシーの初期値
    - price/size を snap するか reject するか
    - snap の丸め方向（down/up/nearest）
    - snap 後の許容乖離（bps or ticks）

## 推奨デフォルト（PM回答が無い場合の暫定）
- D1: 段階導入（Phase 1 から）
- D2: Option A(JSON)
- D3: TOML配列（テンプレ無し、price_mode/size_mode 最小）
- D4: Rust実装 + TOMLパラメータ
- D5: price=snap(nearest), size=snap(down), 許容乖離=小（bpsで設定）

## 次アクション
- PM回答後、impl_guide_002.md を更新し、dev/impl の issue を確定する。
