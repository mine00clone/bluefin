# 作業指示書（Implementation Guide）：Bluefin Pro 自動売買基盤（Rust / SDK利用）

> 目的：**WebSocketでストリーム購読しつつ、RESTで発注/取消できる**自動売買の土台を、ルール厳守で整備する。
> 対象：別AIが実装するための、**手順・粒度・成果物・検証方法が明確**な指示書。

---

## 0. 絶対順守ルール（最優先）

### Environment & Security

* **秘密情報（秘密鍵・トークン等）は `.env` のみ**に置く。**Git管理に絶対入れない**。
* ログ/JSONを共有する場合は **注文ID・アカウント固有情報（アドレス等）を必ずマスク**する（自動マスク関数を用意する）。

### Code Responsibility & Abstraction

* **単一責務**：1モジュール/1関数が1責務を超えない。
* **ハードコード禁止**：実行時に変わる値（環境、URL、銘柄、サイズ、リトライ等）は**必ず外部設定（`config/*.toml` と `.env`）**から注入。
* **ビジネス非依存の命名**：戦略固有語を避け、再利用できる抽象へ（例：`StrategyRunner`、`OrderExecutor`、`OrderStore`）。
* **境界の明示**：core / adapters(API/DB) / orchestration / presentation を分離し、**traitで境界**を作る。

### API関連の実装ルール（最重要）

* **API実装の前に、必ず“生レスポンス”を取得して保存して確認すること。例外なし。**

  1. まず1回叩いて**生のレスポンスを丸ごとログ**
  2. **プロジェクトローカルの raw ディレクトリに保存**
  3. そのファイルを人間（PM/実装AI）が目視確認
  4. **形が確定してから** parse/model/state を実装

> この方針は Bluefin Pro の REST だけでなく **WS受信メッセージにも同様に適用**（まずは生メッセージを保存）。

---

## 1. 参照（Reference）の固定化（最初に実施）

### 1.1 `dev/` に参照Markdownを保存

以下を **必ず** `dev/` 配下に Markdown として作成し、以後の実装で参照する：

* `dev/reference_bluefin_pro_api.md`

  * Bluefin Pro API Reference（ユーザ提示リンク）を貼る（URLはMarkdown内のコードとして保持）
  * 主要Base URLと重要エンドポイントを抜粋してメモ
  * WebSocket購読は `/ws/market` と `/ws/account` を使用する点を明記（後述） ([Bluefin Exchange][1])

* `dev/reference_pro_sdk.md`

  * pro-sdk GitHub（ユーザ提示リンク）を貼る（URLはコードとして保持）
  * OpenAPI spec の bundle/生成手順（redocly bundle / apigen）を抜粋 ([GitHub][2])

> pro-sdk は OpenAPI spec を bundle し、`apigen` で各言語SDKを生成する前提の構成になっているため、将来SDK更新が必要になった時のためにこの手順を残す。 ([GitHub][2])

---

## 2. リポジトリ構成（固定）

要件どおり、以下を含める（無いものは作る）：

```
.
├── .env                      # secrets only (gitignore)
├── config/                   # *.toml (外部設定注入)
├── crates/                   # Rust workspace crates
├── data/                     # sqlite / raw dumps etc (gitignore)
├── dev/                      # reference md 等（作るだけでOK）
├── docs/
│   ├── planning/
│   ├── impl/
│   └── discussion/
├── monitoring/
├── scripts/
├── target/                   # (gitignore)
└── tests/
```

### 2.1 `.gitignore` で必ず除外

* `.env`
* `data/`
* `target/`

---

## 3. Bluefin Pro の最小必須インタフェース（この基盤の“要”）

この基盤では最低限、以下を成立させる：

### 3.1 REST（発注・取消・照会）

* 発注：`POST /api/v1/trade/orders`（SDKでは `TradeApi::post_create_order` 相当） ([Crates][3])
* 取消：`PUT /api/v1/trade/orders/cancel`

  * order hash を **単体 or 配列**で指定可能
  * 指定が無ければ **指定marketの全注文取消**
  * 同一リクエストで取消された注文は同じ time priority 扱い ([Bluefin Exchange][4])
* オープン注文取得：`GET /api/v1/trade/openOrders` ([Bluefin Exchange][5])

### 3.2 WebSocket（状態同期の主軸）

* Market stream：`wss://stream.api.{env}.bluefin.io/ws/market` ([Bluefin Exchange][1])
* Account stream：`wss://stream.api.{env}.bluefin.io/ws/account` ([Bluefin Exchange][6])
* **重要**：取消は REST が ACK しても、成功/失敗は **WSの注文更新で非同期に通知**される前提で設計する。 ([Bluefin Exchange][7])

### 3.3 認証（トークン）

* `POST /auth/v2/token` ([Bluefin Exchange][8])
* `PUT /auth/token/refresh`（**Expiryが5分**と記載あり） ([Bluefin Exchange][9])

---

## 4. Rust SDK選定方針（実装AI向けの固定方針）

### 4.1 SDKは “生成済み” を使う（原則）

* Rust向けには `bluefin-pro` が存在し、内部で `bluefin_api` を利用している（依存関係に `bluefin_api` が含まれる）。 ([Docs.rs][10])
* `bluefin-pro` の `prelude` には **Environment と URL組立**が用意されているため、Base URLハードコードを避けやすい。 ([Docs.rs][10])

### 4.2 将来のSDK更新は pro-sdk 手順で（必要時のみ）

* pro-sdk repo には OpenAPI spec の bundle と、`apigen` 実行手順がある。 ([GitHub][2])
* OpenAPI Generator の Rust generator オプションも参照可能（必要時）。 ([openapi-generator.tech][11])

---

## 5. クレート分割（要求機能を“適切な粒度”で独立）

> 依存関係の向き：**core → traits → adapters**。
> core は reqwest/sqlite/websocket の具体実装を知らない。

### 5.1 crates 構成（提案：この通りに作る）

`crates/` 配下に以下を作成（命名は例、責務は固定）：

1. `bf_config`

* `config/*.toml` と `.env` を読み、型に落とす
* **環境（{env}）・DBパス・WS/REST設定・銘柄等は全てここから注入**

2. `bf_core`

* ドメイン（注文状態、約定、残高スナップショット、イベント）
* **trait定義（境界）**：

  * `OrderExecutor`（発注/取消）
  * `OrderRepository`（現在注文/履歴の永続化）
  * `BalanceRepository`（残高スナップショット）
  * `TradeHistoryRepository`（約定履歴）
  * `StreamSource`（WS購読）

3. `bf_auth`

* `/auth/v2/token` と `/auth/token/refresh` を扱う
* トークンの期限（5分）を考慮し自動更新（refresh）する ([Bluefin Exchange][9])

4. `bf_rest`

* REST 呼び出し（発注/取消/照会/口座系）
* **最初は“生レスポ保存スクリプト”だけ**を提供し、モデル整備は後回し（ルール順守）

5. `bf_ws`

* WS接続（market/account）
* 再接続、ping/pong、購読管理
* 生メッセージ保存（raw）を最初に実装
* `wss://` 利用のため TLS feature 設定が必要（`tokio-tungstenite` はTLSがデフォルト無効のため、featureで有効化する） ([Docs.rs][12])

6. `bf_balance`

* **残高管理機能**（デポジット済み残高、利用可能残高の概念分離）
* WS/RESTの情報を統合し、時系列スナップショットとしてDBへ保存

7. `bf_order_exec`

* **注文（発注・取消）実行機能**

  * 個別取消・バッチ取消（cancelは単体or配列対応、未指定時の全取消も扱う） ([Bluefin Exchange][4])
  * TIF/注文条件：FOK / IOC / GTC 等（TIFは仕様に記載）
  * 取消の成否はWS更新で確定する前提（ACK/確定の2段階） ([Bluefin Exchange][7])

8. `bf_order_manager`

* **注文管理機能（状態機械）**

  * 現在の全注文把握、部分約定/完全約定、現在注文の一覧/部分取得
  * ソート（例：time priority / price / created_at / status）
  * “WSイベント”を唯一の真実（source of truth）として更新し、RESTの openOrders は**起動時の再同期**に使う ([Bluefin Exchange][5])

9. `bf_history`

* **履歴管理**

  * 約定履歴（fills / trades）
  * 未約定注文履歴（出したが未約定・取消・期限切れ等）
  * 指定期間の取引履歴クエリ（DB側で期間検索）

10. `bf_storage_sqlite`

* SQLite 永続化（簡易でOK）
* migration とインデックス（期間検索・注文ID検索・状態検索）

11. `bf_app`（binary）

* orchestration（WS起動、REST実行、state更新、monitoring起動）
* **戦略はここに直書きしない**：`Strategy` trait を作り差し替え可能に

---

## 6. ドキュメント駆動開発（あなたの運用ルールに完全準拠）

### 6.1 Planning Document（docs/planning）

* `docs/planning/planning_001.md` を最初に作る

  * ゴール、段階、主要リスク（WS/REST整合、token期限など）を列挙

### 6.2 Implementation Guide（ルート）

* ルートに `impl_guide_001.md` を作る（＝この指示書をそのまま転記してもよい）
* 管理用：チェックリスト、方針、検証方法、タスク分解の親

### 6.3 Task List（docs/impl）

* `docs/impl/impl_guide_001_issueA_ws_kernel.md` のように issue 単位で作成
* 各タスクファイルには必ず以下を入れる：

  * 影響ファイル一覧（追加/変更）
  * 追加する構造体/trait/関数の名前と責務
  * 例外・エラー方針
  * 受け入れ条件（DoD）
  * 進捗チェックリスト（完了条件がYes/Noで判定可能）

### 6.4 Discussion（docs/discussion）

* 不明点や意思決定が必要な事項は、実装を止めて `docs/discussion/discussion_001.md` を作成
* PM承認後：impl_guide → task list の順で反映

---

## 7. “生レスポンス保存”の具体運用（REST/WS共通）

### 7.1 保存先の標準（推奨）

* **機密が入る可能性が高い**ため、rawは `data/raw/` に保存して gitignore（安全優先）
* テストに使えるように、マスク済みのサンプルのみ `tests/fixtures/` にコピーしてコミット可

例：

* `data/raw/rest/auth_token_20260120.json`
* `data/raw/rest/open_orders_20260120.json`
* `data/raw/ws/account_msg_20260120_ndjson`
* `tests/fixtures/rest/open_orders_masked.json`

### 7.2 “必ず作る” 生取得スクリプト（エンドポイントごと）

* `scripts/` または各crateの `examples/` に、**1エンドポイント=1スクリプト**で作る
* 実装順（最低限）：

  1. `auth token`（`/auth/v2/token`） ([Bluefin Exchange][8])
  2. `token refresh`（`/auth/token/refresh`） ([Bluefin Exchange][9])
  3. `openOrders`（`/trade/openOrders`） ([Bluefin Exchange][5])
  4. `create order`（`/trade/orders`） ([Crates][3])
  5. `cancel orders`（`/trade/orders/cancel`） ([Bluefin Exchange][4])
  6. WS account 接続してメッセージ保存（`/ws/account`） ([Bluefin Exchange][6])
  7. WS market 接続してメッセージ保存（`/ws/market`） ([Bluefin Exchange][1])

---

## 8. 状態管理の基本方針（自動売買の“事故”を防ぐ設計）

### 8.1 Order State は WS を正とする

* REST（openOrders）は「現在値のスナップショット」
* WS（order updates）は「状態遷移のイベント列」
  → **注文管理（部分約定/完全約定/取消）はイベント駆動で更新**し、RESTは再同期用途に限定

### 8.2 取消は二段階

* REST取消：リクエストACK（受付）
* 確定：WSの注文更新で成功/失敗が届く（取消理由もここで返る） ([Bluefin Exchange][7])
  → `bf_order_exec` は「ACK待ち」と「確定待ち」を分けたAPIを提供すること

### 8.3 TIF（注文条件）

* Time-in-Force として **FOK / IOC / GTC** がドキュメントに記載されている
  → `OrderRequest` に `time_in_force` を持たせ、設定注入で切り替え可能にする（戦略直書き禁止）

---

## 9. SQLite 保存（簡易で十分だが “最低限の形” は固定）

### 9.1 保存するもの（最低限）

* `orders`：注文（現在状態、累積約定量、status、timestamps）
* `order_events`：WSイベント（生に近い形で保存、後から再生できるように）
* `fills/trades`：約定履歴
* `balances`：残高スナップショット
* `raw_dumps_index`：rawファイルのメタ（いつ/どのendpoint/マスク有無）

### 9.2 目的

* バグ時に **DB + raw + WSイベント**で再現できる
* “現時点の注文/残高”だけでなく、**履歴から説明可能**になる

---

## 10. 検証方法（DoD：Definition of Done）

### 10.1 最小スモークテスト（人手でOK）

* `.env` と `config/*.toml` を用意
* `bf_ws`：`/ws/account` に接続し、受信を `data/raw/ws/...` に保存できる ([Bluefin Exchange][6])
* `bf_rest`：`openOrders` が取得でき、raw JSON を保存できる ([Bluefin Exchange][5])
* `bf_order_exec`：

  * `create order` を実行し raw を保存
  * `cancel order(s)` を実行し raw を保存（単体・複数・全取消の少なくとも2パターン） ([Bluefin Exchange][4])
  * 取消確定が WS イベントで観測できる（“ACKだけで成功扱いにしない”） ([Bluefin Exchange][7])

### 10.2 自動テスト（最低限）

* `bf_order_manager`：fixture（マスク済みWSイベント）を流して状態遷移を検証
* `bf_storage_sqlite`：migration適用と期間検索の単体テスト

---

## 11. 実装AIへの作業順（この順番を崩さない）

1. **ディレクトリ & gitignore** 整備（要件通り）
2. `dev/reference_*.md` 作成（リンク固定）
3. `docs/planning/planning_001.md` 作成
4. ルート `impl_guide_001.md` 作成（＝本書）
5. `docs/impl/` に issue タスクを切る（WS / REST / storage / order-manager / balance / history）
6. API実装に入る前に **raw取得スクリプト**を各エンドポイントで作成し、raw保存→目視確認
7. WS基盤（接続・再接続・raw保存）
8. REST基盤（raw保存→parse最小→trait実装）
9. 注文実行 → 注文管理（state） → 残高 → 履歴 → SQLite永続化
10. monitoring（ログ整形・マスク・メトリクス）を最後に最低限追加

---

## 付録：実装時に参照すべき“仕様ポイント”まとめ

* Cancel endpoint は **order hash単体/配列**、省略時は **market全取消** ([Bluefin Exchange][4])
* openOrders 取得：`GET /trade/openOrders` ([Bluefin Exchange][5])
* WS account URL：`wss://stream.api.{env}.bluefin.io/ws/account` ([Bluefin Exchange][6])
* WS market URL：`wss://stream.api.{env}.bluefin.io/ws/market` ([Bluefin Exchange][1])
* 取消の成功/失敗（理由含む）は **WS order updates**で通知される前提 ([Bluefin Exchange][7])
* token refresh は expiry 5分の記載あり → 更新前提で設計 ([Bluefin Exchange][9])
* Rust SDK（bluefin-pro）には Environment/URL補助があり、ハードコード回避に使える ([Docs.rs][10])
* `wss://` を使うなら WebSocket ライブラリは TLS feature を有効化する必要がある（tokio-tungstenite） ([Docs.rs][12])

---

[1]: https://bluefin-exchange.readme.io/reference/websocketmarketdata?utm_source=chatgpt.com "/ws/market"
[2]: https://github.com/fireflyprotocol/pro-sdk/tree/main "GitHub - fireflyprotocol/pro-sdk: SDK for trading on bluefin Pro"
[3]: https://crates.io/crates/bluefin_api/1.1.0?utm_source=chatgpt.com "Rust API client for bluefin_api"
[4]: https://bluefin-exchange.readme.io/reference/cancelorders "/trade/orders/cancel"
[5]: https://bluefin-exchange.readme.io/reference/getopenorders "/trade/openOrders"
[6]: https://bluefin-exchange.readme.io/reference/websocketaccountdata "/ws/account"
[7]: https://bluefin-exchange.readme.io/reference/order-creation-cancellations-1?utm_source=chatgpt.com "Order Creation & Cancellations"
[8]: https://bluefin-exchange.readme.io/reference/post_auth-v2-token "/auth/v2/token"
[9]: https://bluefin-exchange.readme.io/reference/put_auth-token-refresh "/auth/token/refresh"
[10]: https://docs.rs/bluefin-pro/latest/bluefin_pro/all.html "List of all items in this crate"
[11]: https://openapi-generator.tech/docs/generators/rust/?utm_source=chatgpt.com "Documentation for the rust Generator"
[12]: https://docs.rs/crate/tokio-tungstenite/latest?utm_source=chatgpt.com "tokio-tungstenite 0.28.0"
