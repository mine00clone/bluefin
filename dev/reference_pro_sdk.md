# Bluefin Pro SDK Reference

## GitHub リポジトリ
```
https://github.com/fireflyprotocol/pro-sdk
```

---

## SDK構成

### 対応言語
- **Rust** (62.7%) - メイン
- Python (22.4%)
- TypeScript (14.0%)

### Rust SDK
- **crate名**: `bluefin-pro`
- **最新バージョン**: 1.5.0 (2026-01-15)
- **docs.rs**: https://docs.rs/bluefin-pro/latest/bluefin_pro/

---

## Rust SDK依存関係

### 暗号化
- blake2, ed25519-dalek, secp256k1, sha2
- sui-crypto

### シリアライゼーション
- bcs, serde, serde_json, hex, base64

### ネットワーク
- reqwest (HTTP)
- tokio-tungstenite (WebSocket) ※dev dependency

### ユーティリティ
- chrono (タイムスタンプ)
- sui-sdk-types

---

## SDK更新手順（将来のAPIスペック更新時）

### 前提条件
```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# OpenAPI Generator CLI
npm install -g @openapitools/openapi-generator-cli
```

### OpenAPI spec bundle
```bash
# pro-sdk リポジトリ内で
npx @redocly/cli bundle openapi/bluefin_api.yaml -o openapi/bundled/bluefin_api.yaml
```

### SDK生成
```bash
cargo run --bin apigen -- --lang rust
```

---

## bluefin-pro crate使用例

### Cargo.toml
```toml
[dependencies]
bluefin-pro = "1.5"
tokio = { version = "1", features = ["full"] }
```

### 環境別URL取得
```rust
use bluefin_pro::prelude::*;

// Environment enum で staging/prod を切り替え
// URL組み立てユーティリティあり
```

---

## 注意点

### TLS設定
`wss://` を使う場合、tokio-tungstenite は TLS がデフォルト無効。
feature で有効化が必要:
```toml
tokio-tungstenite = { version = "0.28", features = ["native-tls"] }
# または
tokio-tungstenite = { version = "0.28", features = ["rustls-tls-native-roots"] }
```

### ドキュメント
docs.rs のドキュメントカバレッジは 4.29% と低い。
実装時は各言語ディレクトリの README と OpenAPI spec を参照。
