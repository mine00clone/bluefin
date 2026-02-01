# discussion_002 - 戦略として柔軟に運用できる「注文/設定/marketメタ」のゼロベース設計

## 目的（ユーザ意図の反映）
現状は「単発の raw スクリプトで注文を投げる」段階を超え、
**複数銘柄・複数注文・複数戦略**を運用し、環境切替やパラメータ変更を安全かつ高速に回せる基盤が必要。

そのために、
- 何をどのファイルに持つべきか
- どの責務をどの層に分離すべきか
- market 仕様差（tick/step/min/max 等）をどう吸収すべきか
- 注文を「戦略」として運用できるようにするには何が必要か

を**ゼロベース**で整理し、設計を決めるための論点をまとめる。

注: `.env` は secrets only であり、ここでは詳細は省略する。

---

## 現状の問題（整理）

### 1) 設定が単発実行向けで、戦略運用に耐えない
- `config/default.toml` に `[orders.create]` / `[orders.cancel]` を置き、
  「どの銘柄を」「どんな注文で」まで固定している。
- 複数銘柄・複数注文（同時・条件分岐・段階発注）を行うたびに `config/default.toml` を書き換える必要があり、
  運用上の摩擦・事故要因が大きい。

### 2) market 仕様差（tick/step/min 等）を吸収できていない
- `tickSizeE9` に合わない価格を出して reject/cancel された事例がある。
- これは「銘柄ごとに異なる制約」を
  - 戦略
  - 注文定義
  - 実行
  のどこかで必ず吸収しなければならないことを意味する。

### 3) 実行時に market info を毎回取りに行くのは遅い
- 正規化に必要な情報を注文ごとにAPI取得すると、
  - 遅い
  - 不安定
  - 実行時に不利
- これらは頻繁に変わらない前提なので、基本は **ローカルスナップショット**（または起動時キャッシュ）にすべき。

### 4) 設定変更が「ビルド」や「コード改変」に寄ると最悪
- postOnly 等の切替がコード・ビルドに依存する形は運用不能。
- 切替は「設定ファイル差し替え」や「run config差し替え」で行うべき。

---

## ゼロベース要件（こうしたい）

### A. 設定の分離
- **アプリ共通設定**（環境、URL、DB、ログ、監視）
- **市場定義/仕様**（tick/step/min/max、許容注文タイプ等）
- **監視対象**（どの銘柄を見るか）
- **戦略定義**（シグナル生成、条件、パラメータ）
- **注文計画/ルール**（指値/成行、オフセット、段階、上限）
- **リスク制約**（最大建玉、最大注文回数、1注文上限、kill switch）
- **実行モード**（dry-run / live、paper、許可フラグ）

これらは、1ファイルに詰め込まず役割ごとに分離して差し替え可能にしたい。

### B. 「戦略」と「実行」を明確分離
- 戦略は「何をしたいか（Intent）」を生成する。
- 実行は「どうやって取引所制約に合わせて実現するか（Normalize/Sign/Send/Confirm）」を担う。

### C. market制約の吸収は Executor 側で一元化
- tick/step/min/max は銘柄ごと。
- 戦略がそれを知ると設計が汚れる。
- **Executor が必ず正規化**し、
  - 送信前に fail-fast
  - あるいは丸め（snap）
  のポリシーを統一する。

### D. 運用で必要な「状態」と「確認フロー」
- REST ACK と WS 確定（注文更新）を分けて扱う。
- openOrders は補助（スナップショット）であり、WS を正にするなら確認フローを標準化する。
- raw 取得は開発用だが、運用でもトラブル時に再現できる形が望ましい。

---

## 提案するディレクトリ構成（完全版案）

```
.
├── config/
│   ├── app.toml                 # アプリ共通（env/url/db/log/paths）
│   ├── profiles/
│   │   ├── prod.toml             # 環境ごとの差分（URL/WSなど）
│   │   └── staging.toml
│   ├── markets/
│   │   ├── snapshot.json         # 市場仕様スナップショット（tick/step/min/max...）
│   │   └── README.md             # 更新方針
│   ├── universe/
│   │   ├── universe_default.toml # 監視対象（銘柄群）
│   │   └── universe_*.toml
│   ├── strategies/
│   │   ├── strategy_grid.toml    # 戦略パラメータ
│   │   └── strategy_*.toml
│   ├── plans/
│   │   ├── plan_paper.toml       # 注文計画（複数注文/テンプレ/階層）
│   │   └── plan_live.toml
│   ├── risk/
│   │   ├── risk_default.toml     # 口座/戦略共通の制約
│   │   └── risk_*.toml
│   └── run/
│       ├── run_paper.toml        # 実行モード選択・どのファイルを使うか
│       └── run_live.toml
├── data/
│   ├── bluefin.db
│   └── raw/
├── crates/
├── dev/
├── scripts/
└── tests/
```

### 各ファイルの役割（要点）
- `config/app.toml`: すべての run に共通の土台。パス/ログ/DB 等。
- `config/profiles/*.toml`: 環境別URL/WS等の差分。
- `config/markets/snapshot.json`: market制約の単一ソース（実行時通信を避ける）。
- `config/universe/*.toml`: 監視対象銘柄の集合。
- `config/strategies/*.toml`: 戦略パラメータ。
- `config/plans/*.toml`: 注文を「テンプレ＋複数定義」で表現。
- `config/risk/*.toml`: 最大サイズ・最大建玉・連続発注制限・kill switch等。
- `config/run/*.toml`: どの組み合わせで動かすか（app/profile/universe/strategy/plan/riskの参照）。

---

## 「注文を戦略として運用する」ためのモデル（提案）

### 1) Strategy が出すのは Order ではなく Intent
例: 
- 「BTC-PERP を買いたい」
- 「現在値から -300bps の指値で、0.001 BTC」
- 「一定時間で未約定ならキャンセル」

これをそのまま取引所の `CreateOrderRequest` にせず、
**ドメイン層の Intent（戦略の意図）**として表現する。

### 2) Executor が MarketMetadata を使って正規化
- market の tick/step/min/max
- 価格/数量の丸め（snap / reject）方針
- postOnly/reduceOnly/TIF の制約

を Executor に集約し、戦略から隠蔽する。

### 3) Plan と Risk を別にして、戦略を安全にする
- Plan: どんな注文パターンを許すか（段階、割合、offset、再発注条件など）
- Risk: どれくらいまで許すか（上限、連続、同時数、想定損失など）

戦略は「機会」を出すだけで、
最終的に発注できるかは Plan+Risk のゲートを通す。

---

## Marketメタデータの持ち方（設計オプション）

### Option A: ローカルスナップショット（推奨）
- `config/markets/snapshot.json` を正とする
- `scripts/update_markets_snapshot.sh` で手動更新
- 実行はスナップショット参照のみ

長所:
- 実行が速い/安定
- ネットワーク依存が減る

短所:
- 更新運用が必要

### Option B: 起動時に一度だけ取得してキャッシュ（SQLite or file）
- 起動で `exchange/info` を取得
- `data/market_snapshot.json` に保存
- TTL/バージョンで更新

長所:
- 更新が自動

短所:
- 起動時にネットワーク依存

---

## 現在の実装・設定との対応表（現状把握）

### ディレクトリ（実体）
- `config/default.toml`: いまは単一ファイルに多くの責務が集中
- `crates/bf_config/src/lib.rs`: `AppConfig` が `default.toml` を丸ごと deserialize
- `crates/bf_rest/examples/create_order_raw.rs`:
  - 現在は `orders.create` を読んで単発の注文を作り、
  - 実行時に `get_market_ticker` + `exchange::info::markets` に依存する（通信あり）
- `scripts/raw_*.sh`: examples を呼ぶだけの薄いラッパ

### 現状の設定（抜粋）
- `config/default.toml`:
  - `[markets].symbols`: 監視対象っぽいが、実際は注文にも流用されがち
  - `[orders]`: 実行フラグ/注文フラグ/レバレッジ
  - `[orders.create]`: 単一注文
  - `[orders.cancel]`: 単一キャンセル

---

## 次に決めたいこと（意思決定が必要）

1) 提案構成（app/profile/markets/universe/strategies/plans/risk/run）の採用範囲
- いきなり完全分離か、段階導入（まず markets+plans+run だけ等）か

2) market snapshot の形式
- JSON/TOML/SQLite のどれにするか

3) Plan の表現
- 複数注文を TOML 配列で持つ
- テンプレ/変数（offset bps / size / price mode）をどう表現するか

4) Strategy の粒度
- strategy は Rust 実装として固定で、パラメータだけ TOML
- strategy 自体を DSL/条件ファイルで表現するか

---

## 参考: 最小の段階導入案（摩擦を減らす）

Phase 1:
- `config/markets/snapshot.json` を導入（実行時通信ゼロ）
- `config/plans/plan_*.toml` を導入（複数注文を定義可能）
- `config/run/run_*.toml` を導入（どの plan を使うか選べる）

Phase 2:
- universe/strategies/risk を分離
- OMS/Balance/History と統合し、戦略運用へ
