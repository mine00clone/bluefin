# Bluefin Pro API Reference

## 公式ドキュメント
```
https://bluefin-exchange.readme.io/reference/welcome-to-bluefin-pro
```

---

## Base URLs

### REST API
- **Staging**: `https://trade.api.staging.bluefin.io`
- **Production**: `https://trade.api.prod.bluefin.io`
- **Exchange Data**: `https://api.{env}.bluefin.io`

### WebSocket
- **Market Stream**: `wss://stream.api.{env}.bluefin.io/ws/market`
- **Account Stream**: `wss://stream.api.{env}.bluefin.io/ws/account`

---

## 主要エンドポイント

### 認証 (Authentication)
| Method | Endpoint | 説明 |
|--------|----------|------|
| POST | `/auth/v2/token` | トークン取得 |
| PUT | `/auth/token/refresh` | トークン更新（expiry: 5分） |

### 取引 (Trading)
| Method | Endpoint | 説明 |
|--------|----------|------|
| POST | `/api/v1/trade/orders` | 注文作成 |
| PUT | `/api/v1/trade/orders/cancel` | 注文キャンセル |
| GET | `/api/v1/trade/openOrders` | オープン注文取得 |
| PUT | `/api/v1/trade/leverage` | レバレッジ更新 |
| PUT | `/api/v1/trade/adjustIsolatedMargin` | 分離マージン調整 |
| POST | `/api/v1/trade/withdraw` | 出金 |

### アカウント (Account)
| Method | Endpoint | 説明 |
|--------|----------|------|
| GET | `/api/v1/account` | アカウント情報 |
| GET | `/api/v1/account/trades` | 取引履歴 |
| GET | `/api/v1/account/transactions` | トランザクション履歴 |
| GET | `/api/v1/account/fundingRateHistory` | ファンディングレート履歴 |

### 取引所データ (Exchange)
| Method | Endpoint | 説明 |
|--------|----------|------|
| GET | `/api/v1/exchange/info` | 取引所情報 |
| GET | `/api/v1/exchange/depth` | オーダーブック |
| GET | `/api/v1/exchange/trades` | 最近の取引 |
| GET | `/api/v1/exchange/ticker` | ティッカー（単一） |
| GET | `/api/v1/exchange/tickers` | ティッカー（全て） |
| GET | `/api/v1/exchange/candlesticks` | ローソク足 |

---

## 注文キャンセル仕様

`PUT /api/v1/trade/orders/cancel` の仕様:
- `orderHashes`: order hash を**単体または配列**で指定可能
- `orderHashes`省略 + `market`指定: そのmarketの**全注文キャンセル**
- 同一リクエストでキャンセルされた注文は同一の time priority 扱い
- **重要**: キャンセルの成功/失敗はWSの注文更新で非同期に通知される

---

## 注文条件 (Time-in-Force)

| TIF | 説明 |
|-----|------|
| GTC | Good-Til-Cancel（キャンセルまで有効） |
| IOC | Immediate-Or-Cancel（即時約定、残りはキャンセル） |
| FOK | Fill-Or-Kill（全量約定または全キャンセル） |

---

## 注意点（実運用で確認された事項）
- **SignedAtUtcMillis の制約**: サーバは `signedAtMillis` が「1分以上過去」の場合に `400 Bad Request` を返す。
  - HTTP raw のエラーボディ例: `Invalid Payload: SignedAtUtcMillis must be no earlier than 1 minute in the past`
  - 対策: **サーバの Date ヘッダ由来の時刻**を `signedAtMillis` に使用するか、送信直前に生成する。
- **SelfTradePreventionType**: `MAKER` 指定が 400 になるケースがあるため、**UNSPECIFIED での送信が通ることを確認**。
- **openOrders のレスポンス揺れ**: `createTime` が文字列で返るケースがあり、型デシリアライズが壊れることがある。実装前に raw を保存してから型を確定する。
- **cancelAll のレスポンス揺れ**: `CancelAllResponse { canceled_count }` ではなく、配列っぽい戻りになるケースがある。実装前に raw を保存してから型を確定する。
- **cancelAll の HTTP 応答**: `202 Accepted` でボディが空のケースがある（HTTP raw で確認済み）。ACK と WS 確定を分離する前提で扱う。

---

## WebSocket購読

### Market Stream (`/ws/market`)
- オーダーブック更新
- 最近の取引
- ティッカー更新

### Account Stream (`/ws/account`)
- 注文更新（作成、約定、キャンセル）
- ポジション更新
- 残高更新

**重要**: 取消の成功/失敗（理由含む）はAccount StreamのWS order updatesで通知される
