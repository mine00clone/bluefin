# 001 - Bluefin Pro SDK 実装状態

## 概要

Bluefin Pro取引所向けRust SDKベースの自動売買基盤の実装状況。
staging環境での動作確認済み。

---

## 実装済みクレート

### bf_config
**状態**: 基本実装完了 + Phase1の設定分割を追加

- `config/default.toml`からの設定読み込み
- `.env`からのシークレット読み込み
- `Environment`（Staging/Prod）の定義
- `run.toml` を入口にした config 分割（app/profile/plan/markets）
- market snapshot の生成フロー（raw→snapshot）を追加

### bf_core
**状態**: 基本実装完了 + 市場メタ/Planモデル追加

ドメイン型とトレイト定義:

| 型 | 説明 |
|---|---|
| `Order` | 注文状態 |
| `OrderRequest` | 注文リクエスト |
| `CancelRequest` | 取消リクエスト（Single/Batch/AllForMarket） |
| `Side` | Buy/Sell |
| `OrderType` | Limit/Market |
| `TimeInForce` | GTC/IOC/FOK |
| `OrderStatus` | Open/Partial/Filled/Cancelled/Expired/Rejected |
| `Balance` | 残高 |
| `Fill` | 約定 |
| `MarketMeta` | 市場制約（tick/step/min等） |
| `Plan` | 複数注文のPlan（Phase1最小） |
| `redact_id` | ログ用IDマスキング |

トレイト:

| トレイト | 説明 |
|---|---|
| `OrderExecutor` | 注文発行・取消 |
| `OrderRepository` | 注文永続化 |
| `BalanceRepository` | 残高永続化 |
| `TradeHistoryRepository` | 約定履歴永続化 |
| `StreamSource` | WS購読 |

### bf_auth
**状態**: 実装完了・動作確認済み

```rust
// 使用例
let token_manager = TokenManager::new(
    private_key_hex,
    account_address,
    BluefinEnvironment::Staging,
)?;

// テストアカウントを使う場合
let token_manager = TokenManager::with_test_keys(BluefinEnvironment::Staging)?;

// トークン取得（自動更新）
let token = token_manager.get_token().await?;
```

機能:
- Ed25519署名によるSuiウォレット認証
- JWTトークン取得（5分有効）
- 自動トークン更新（/auth/token/refresh 対応）
- SDKテストアカウント対応

### bf_order_exec
**状態**: 実装完了・動作確認済み

```rust
// 使用例
let executor = BluefinOrderExecutor::new(
    token_manager.clone(),
    config.orders.default_leverage,
);

// 注文作成
let order_request = OrderRequest::limit(
    "ETH-PERP",
    Side::Buy,
    dec!(3000),
    dec!(1),
).with_post_only(true);

let order = executor.create_order(order_request).await?;
println!("Order hash: {}", order.order_hash);

// 取消
executor.cancel_order("ETH-PERP", &order.order_hash).await?;
executor.cancel_all("ETH-PERP").await?;
```

機能:
- 注文作成（Limit/Market）
- 注文署名（Ed25519）
- TIF対応（GTC→GTT変換）
- 単一/バッチ/マーケット全取消
- IDS_IDキャッシュ
- Decimal→e9変換

### bf_ws
**状態**: WsClient 実装完了 + raw 取得 examples 動作確認済み

機能:
- Market WebSocket接続（WsClient）
- Account WebSocket接続（認証付き, WsClient）
- Subscription/Unsubscription（WsClient）
- Ping/Pong対応（WsClient）
- raw 保存（WsClient）

### bf_rest
**状態**: スケルトン実装

---

## 動作確認済みAPI

### 認証
- `/auth/v2/token` - トークン取得 ✅

### Public REST
- `/api/v1/exchangeInfo` - 取引所情報 ✅
- `/api/v1/ticker` - ティッカー ✅
- `/api/v1/depth` - 板情報 ✅

### Private REST
- `/trade/openOrders` - オープン注文取得 ✅
- `/trade/orders` - 注文作成 ✅
- `/trade/orders/cancel` - 注文取消 ✅
- `/api/v1/account` - アカウント詳細（存在するアカウントのみ）✅

### WebSocket
- `wss://stream.api.sui-staging.bluefin.io/ws/market` - マーケットストリーム ✅
- `wss://stream.api.sui-staging.bluefin.io/ws/account` - アカウントストリーム ✅

---

## 未実装クレート

### bf_order_manager
**状態**: 未実装

予定機能:
- 注文状態管理（HashMap<OrderHash, Order>）
- WSイベントによる状態遷移
- 部分約定→残量更新→完全約定
- REST openOrdersでの再同期

### bf_balance
**状態**: 未実装

予定機能:
- 残高取得（REST）
- WS差分更新
- スナップショット保存

### bf_history
**状態**: 未実装

予定機能:
- 約定履歴（/account/trades）
- 期間検索

### bf_storage_sqlite
**状態**: 未実装

予定機能:
- orders, order_events, fills, balancesテーブル
- migration
- 期間検索

### bf_app
**状態**: 未実装

予定機能:
- 全コンポーネントのオーケストレーション
- Strategy trait

---

## 重要な技術的詳細

### 環境切り替え
```rust
// Staging
let environment = BluefinEnvironment::Staging;

// Production
let environment = BluefinEnvironment::Production;
```

URL自動解決:
- `auth::url(env)` → 認証エンドポイント
- `trade::url(env)` → 取引エンドポイント
- `account::url(env)` → アカウントエンドポイント
- `ws::market::url(env)` → マーケットWS
- `ws::account::url(env)` → アカウントWS

### Private Key形式
- 64文字hex形式（`0x`プレフィックスなし）
- Bech32形式（`suiprivk...`）は非対応

### e9フォーマット
Bluefin APIは全ての数値をe9形式（10^9倍）で扱う:
```rust
// 100 USD → "100000000000"
let price_e9 = (price * 1_000_000_000).to_string();
```

### TimeInForce
- 内部: `GTC` (Good-Til-Cancel)
- SDK: `GTT` (Good-Til-Time)
- 自動変換済み

### 取消API
- `order_hashes`指定: 特定注文を取消
- `order_hashes`省略: マーケット全注文を取消
- 成否はWSで確定

---

## Examples

```bash
# 認証テスト
cargo run --example get_token -p bf_auth

# Public REST
cargo run --example public_read -p bf_rest

# Private REST
cargo run --example private_read -p bf_rest

# Market WebSocket
cargo run --example market_stream -p bf_ws

# Account WebSocket
cargo run --example account_stream -p bf_ws

# 注文作成・取消
cargo run --example create_order -p bf_order_exec
```

---

## ファイル構成

```
crates/
├── bf_config/          # 設定管理
├── bf_core/            # ドメイン型・トレイト
├── bf_auth/            # 認証・トークン管理
│   └── examples/
│       └── get_token.rs
├── bf_rest/            # REST API
│   └── examples/
│       ├── public_read.rs
│       └── private_read.rs
├── bf_ws/              # WebSocket
│   └── examples/
│       ├── market_stream.rs
│       └── account_stream.rs
├── bf_order_exec/      # 注文執行
│   └── examples/
│       └── create_order.rs
├── bf_order_manager/   # (未実装)
├── bf_balance/         # (未実装)
├── bf_history/         # (未実装)
├── bf_storage_sqlite/  # (未実装)
└── bf_app/             # (未実装)
```

---

## 次のステップ

1. **bf_order_manager** - WSからの注文更新を処理し状態管理
2. **bf_balance** - 残高追跡
3. **bf_history** - 約定履歴
4. **bf_storage_sqlite** - 永続化
5. **bf_app** - 統合・オーケストレーション

---

## 更新履歴

- 2026-01-21: 初版作成
  - bf_auth実装完了
  - bf_order_exec実装完了
  - API動作確認完了
- 2026-02-02: impl_guide_002 (A/B/C/E) 反映 + market snapshot 生成
  - `cargo test --workspace` を実行し全テストPASS
  - exchange_info raw → snapshot 生成フロー追加
