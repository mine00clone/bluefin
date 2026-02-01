以下の方針で「Bluefin Proで自動売買するためのRust基盤」を、**WSストリーム＋REST注文**を中心に、機能ごとに**独立クレート**として分割した形で整備しました（まずは拡張しやすい“骨格”＝scaffold を作成）。
Bluefin Proは **RESTが `trade.api.{env}.bluefin.io` / `api.{env}.bluefin.io` 系、WSが `stream.api.{env}.bluefin.io`** で分かれているため、クレート分割もこの境界を強く意識しています。 ([Bluefin Exchange][1])

---

## 1. Bluefin Proの接続面（REST/WS）の前提整理

### REST（注文・注文キャンセル・オープンオーダー）

* **新規注文**: `POST /api/v1/trade/orders`（`https://trade.api.{env}.bluefin.io/api/v1/trade/orders`） ([Bluefin Exchange][1])
* **注文キャンセル**: `PUT /api/v1/trade/orders/cancel`（`https://trade.api.{env}.bluefin.io/api/v1/trade/orders/cancel`） ([Bluefin Exchange][2])

  * キャンセルは **order hash を単体またはリストで指定可能**
  * **marketを指定して order hash未指定なら、そのmarketの全注文キャンセル**
  * **同一アカウントの注文である必要**、同一リクエストでキャンセルされた注文は同一の優先度扱い…などの性質が明記されています ([Bluefin Exchange][2])
* **オープンオーダー取得**: `GET /api/v1/trade/openOrders`（`https://trade.api.{env}.bluefin.io/api/v1/trade/openOrders`） ([Bluefin Exchange][3])

### WS（マーケット／アカウント）

* **マーケットWS**: `wss://stream.api.{env}.bluefin.io/ws/market` ([Bluefin Exchange][4])
* **アカウントWS**: `wss://stream.api.{env}.bluefin.io/ws/account` ([Bluefin Exchange][5])

### SDK（Rust）

* `fireflyprotocol/pro-sdk` がOpenAPIから各言語SDKを生成する前提で管理されており、OpenAPI spec のbundleや生成ツールの記述があります。 ([GitHub][6])
* Rust側は `bluefin-pro` crate が存在し、`bluefin_api` を依存に持ちつつ、環境別URLやWS URLのユーティリティ等が見えています（`prelude::trade::url` / `prelude::ws::*::url` など）。 ([Docs.rs][7])

---

## 2. 目標要件を満たすアーキテクチャ方針

要件（WSでストリーム、RESTで注文、残高・注文・履歴・SQLite保存、クレート分割）を満たすために、以下の役割分担を固定します。

### 基本戦略（実運用で破綻しにくい形）

1. **起動時にRESTでスナップショット取得**（残高・オープンオーダー）
2. **WS接続して差分イベントを受信**（注文更新/約定/残高更新など）
3. **内部状態（in-memory）を更新**し、同時に **イベントをSQLiteへ永続化**
4. WS取りこぼし・順序ズレ対策として、**定期的にRESTでreconcile**（`/trade/openOrders` などで整合） ([Bluefin Exchange][3])

> WSのメッセージ仕様（subscribe payload等）はReadme.ioがJS依存でここでは確定できないため、**WS受信は raw JSON → “内部イベント”への変換（Decoder）を差し替え可能**にし、仕様確定後に埋める設計にしています。

---

## 3. 生成したリポジトリ骨格（指定ディレクトリ構成を反映）

ご指定のディレクトリをすべて含み、`data/` と `target/` はgitignore済み、ルートに `.env` を配置、設定は `config/*.toml` から注入、さらに **最初に提示されたリンクは `dev/` に .mdで記録**しています。

**ダウンロード**: [bluefin-rust-autotrader.zip](sandbox:/mnt/data/bluefin-rust-autotrader.zip)

（中身のツリーは作成済みで、以下を含みます）

* `config/`（TOML設定）
* `crates/`（機能別クレート群）
* `data/`（SQLite置き場・gitignore）
* `dev/`（参照リンクの記録md）
* `docs/`（アーキテクチャ）
* `monitoring/`（監視置き場）
* `scripts/`（初期化等）
* `target/`（gitignore）
* `tests/`

---

## 4. クレート分割（要件に沿った“適切な粒度”）

この分割は「APIを叩くだけ」ではなく、**自動売買に必要な情報支援（状態・整合・履歴・検索/ソート）**を作るための境界です。

### `bf-types`（純粋ドメイン）

* 残高、注文、約定、イベント、検索条件、ソートキー等の型
* **TimeInForce（FOK/IOC/GTCなど）**や ReduceOnly/PostOnly等のフラグを“内部表現”として定義
* ここは **I/O禁止（serdeはOK）**にして全クレートが使える核にする

### `bf-config`

* `config/default.toml` + `.env`（dotenv）で設定注入
* `env.name`（例: `staging`）と `use_colocated` を持たせ、SDK側のURLユーティリティにも寄せられる構造にしています（`bluefin-pro` には環境別URL/colocated URLが存在）。 ([Docs.rs][7])

### `bf-auth`

* トークン取得／更新（`/auth/v2/token`, `/auth/token/refresh`）を責務化（内部はSDK利用想定） ([Bluefin Exchange][8])
* **TokenManager**を用意し、REST/WS両方で使えるようにする（WSにもヘッダ認証欄があるため） ([Bluefin Exchange][5])

### `bf-rest`

* **RESTの薄いFacade**
* `create_order / cancel_orders / get_open_orders / get_account_trades ...` のような用途別APIを提供
* 内部では `bluefin-pro`（+ `bluefin_api`）利用を想定（このcrateが依存していることがdocsに見えます）。 ([Docs.rs][7])

### `bf-ws`

* WS接続（market/account）＋再接続＋購読メッセージ送信
* **受信は raw JSON（serde_json::Value）をまず流し、Decoderで内部イベントへ変換**
* MarketとAccountはURLも用途も違うので、同一抽象で扱いつつ接続先だけ変える（`/ws/market` と `/ws/account`）。 ([Bluefin Exchange][4])

### `bf-account`（残高管理）

* 「今のデポジット済み残高はいくらか」＝自動売買に必要な“意味のある残高指標”を提供
* RESTスナップショット＋WS差分で更新、**SQLiteへスナップショット or イベント保存**
* 利用側が欲しいのは「available」「equity」「margin」等になりがちなので、**計算済みのView**をここで返す

### `bf-orders-exec`（注文・削除の実行）

* **単発・バッチ両対応**
* キャンセルは、仕様通り **order hash 単体/リスト/全キャンセル** を吸収してAPI化 ([Bluefin Exchange][2])
* 複数条件（例: FOK/IOC…）は `bf-types` の内部表現 → APIリクエストへマッピングする層にする

### `bf-orders-oms`（注文管理）

* “今どんな注文が出ているか”を保持
* **部分約定 / 完全約定 / キャンセル / 失効**などの遷移を管理
* 一覧取得・部分取得（filter）・ソートをここで提供
* **定期reconcile**で `/trade/openOrders` と整合させ、WS取りこぼしの影響を限定 ([Bluefin Exchange][3])

### `bf-history`（履歴管理）

* 約定済み履歴（fills/trades）＋未約定注文履歴（OMSイベント由来）を統合して扱えるようにする
* 期間指定の取引履歴は `/account/trades` などのREST取得を同期してDBへ保存（APIに存在）。 ([Bluefin Exchange][8])

### `bf-storage`（SQLite）

* SQLiteスキーマ・マイグレーション・保存API（`append_event` / `upsert_order` / `insert_fill` 等）
* “結果は簡易的にSQLiteで十分”を素直に実装する層

### `bf-monitoring`

* `tracing` / metrics（prometheus等）をまとめる
* 「状態が壊れてないか」「WS再接続回数」「注文失敗率」などを出せるようにする

### `bf-app`（実行バイナリ）

* 設定読込 → REST/WS/Auth/Storage を配線 → 各Manager起動 → Strategy（ここは後で差し替え）

---

## 5. 重要な“情報支援”の具体像（API叩くだけで終わらせない）

### 残高管理（`bf-account`）

* **口座残高（deposit済み）**・**利用可能残高**・**証拠金**など、戦略が欲しい形に整形して返す
* “最後に受けたWSイベント時刻”と“最後にRESTでreconcileした時刻”を持ち、異常検知に使う

### 注文実行（`bf-orders-exec`）

* `place(order)` / `place_batch(orders)`
* `cancel_by_hash(hash)` / `cancel_by_hashes(vec)` / `cancel_all(market)`

  * cancelの仕様をそのまま支える（hash単体/リスト/未指定で全キャンセル）。 ([Bluefin Exchange][2])
* 条件（FOK/IOC等）や reduce-only などは内部フラグで統一管理し、APIの表現に落とす

### 注文管理（`bf-orders-oms`）

* 内部で `HashMap<OrderId, OrderState>` を持ち、イベント適用で状態遷移
* **部分約定→残量更新→完全約定**を正しく扱う
* 取得API例：

  * `list_open_orders(filter, sort)`
  * `get_order(order_id)`
  * `list_orders_by_market(market, status, sort)`
* 起動直後の“正”は `/trade/openOrders` にして初期化・再同期に使う ([Bluefin Exchange][3])

### 履歴管理（`bf-history`）

* `/account/trades` などで **約定履歴の期間取得**を行いDBへ保存（APIに存在） ([Bluefin Exchange][8])
* 未約定注文の履歴は OMS がイベントとして保存しているため、それを“注文履歴”として参照できる

---

## 6. scaffold 内で既に行っていること

### (A) 指定のディレクトリ構成＋gitignore

* `data/` と `target/` はgitignore
* ルート `.env` はgitignore
* `config/*.toml` で外部注入

### (B) devに参照リンクのmdを作成

* `dev/bluefin_pro_reference.md` に

  * Bluefin Pro API Reference
  * `fireflyprotocol/pro-sdk`
    を記録済み（実装時はここを“索引”にして参照する運用を想定） ([Bluefin Exchange][9])

---

## 7. 次にやるべき実装タスク（この骨格に追記していく順番）

1. **`bf-rest` に bluefin-pro SDK を組み込む**

   * `bluefin-pro` は環境別URLや `bluefin_api` 依存があるので、ここを中核にする ([Docs.rs][7])
2. **`bf-ws` の subscribe payload / event schema を確定**

   * まずは raw JSON を保存して、仕様確定後に Decoder を実装
3. **`bf-storage` のSQLiteスキーマ確定**

   * 例：`orders`, `order_events`, `fills`, `balances_snapshots`, `positions_snapshots`
4. **reconcile ループ（OMS/Account）**

   * `/trade/openOrders` を基準に整合する処理を入れる ([Bluefin Exchange][3])
5. **注文条件（FOK/IOC等）のAPIマッピング**

   * `bf-types` の内部表現 → RESTリクエストへ変換（ここがバグりやすいのでテスト厚め）

---

必要なら、次のメッセージでこの scaffold 上に「最初の実装ターゲット」を固定して、

* `bf-rest` の `create_order/cancel/get_open_orders` をまず通す
* `bf-ws` は接続・再接続・rawログ保存まで
  …の“最短で動く縦切り”を、具体的なファイル単位で書き進める手順まで落とします。

[1]: https://bluefin-exchange.readme.io/reference/postcreateorder "https://bluefin-exchange.readme.io/reference/postcreateorder"
[2]: https://bluefin-exchange.readme.io/reference/cancelorders "https://bluefin-exchange.readme.io/reference/cancelorders"
[3]: https://bluefin-exchange.readme.io/reference/getopenorders "https://bluefin-exchange.readme.io/reference/getopenorders"
[4]: https://bluefin-exchange.readme.io/reference/websocketmarketdata "https://bluefin-exchange.readme.io/reference/websocketmarketdata"
[5]: https://bluefin-exchange.readme.io/reference/websocketaccountdata "https://bluefin-exchange.readme.io/reference/websocketaccountdata"
[6]: https://github.com/fireflyprotocol/pro-sdk/tree/main "https://github.com/fireflyprotocol/pro-sdk/tree/main"
[7]: https://docs.rs/bluefin-pro/latest/bluefin_pro/all.html "https://docs.rs/bluefin-pro/latest/bluefin_pro/all.html"
[8]: https://bluefin-exchange.readme.io/reference/post_auth-v2-token "https://bluefin-exchange.readme.io/reference/post_auth-v2-token"
[9]: https://bluefin-exchange.readme.io/reference/welcome-to-bluefin-pro "https://bluefin-exchange.readme.io/reference/welcome-to-bluefin-pro"
