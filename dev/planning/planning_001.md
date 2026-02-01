# 001 - Bluefin Pro 自動売買基盤構築

## ゴール
Rust SDKを使用してBluefin Proで自動売買を実行するための基盤を構築する。
- WSでストリーム購読
- RESTで注文発注/取消
- 残高・注文・履歴の管理
- SQLiteでの永続化

## 段階

### Phase 1: 基盤整備 (完了)
- [x] ディレクトリ構成
- [x] gitignore
- [x] 参照ドキュメント (dev/reference_*.md)
- [x] Cargo workspace
- [x] 11クレート骨格
- [x] config/default.toml

### Phase 2: API生レスポンス取得 (次)
- [ ] 各エンドポイントの生レスポンス取得スクリプト作成
- [ ] raw JSONをdata/raw/に保存
- [ ] 目視確認後、パーサー実装開始

### Phase 3: 認証基盤
- [ ] bf_auth: トークン取得・自動更新
- [ ] TokenManager実装

### Phase 4: REST/WS実装
- [ ] bf_rest: SDK統合、注文API
- [ ] bf_ws: 接続、再接続、raw保存

### Phase 5: 状態管理
- [ ] bf_order_manager: 注文状態機械
- [ ] bf_balance: 残高管理
- [ ] reconcileロジック

### Phase 6: 履歴・永続化
- [ ] bf_history: 履歴クエリ
- [ ] bf_storage_sqlite: CRUD完成

### Phase 7: 統合・テスト
- [ ] bf_app: コンポーネント配線
- [ ] スモークテスト
- [ ] 自動テスト

## 主要リスク

### WS/REST整合性
- WSメッセージの取りこぼし
- **対策**: 定期的なREST reconcile

### トークン期限
- 5分でexpiry
- **対策**: TokenManagerで自動更新

### 取消の二段階性
- REST ACK ≠ 取消確定
- **対策**: WS確定待ちを分離設計

## 重要な設計決定

1. **WSが正（source of truth）** - 状態更新はWSイベント駆動
2. **RESTは補助** - スナップショット取得とreconcile用
3. **生レスポンス優先** - API実装前に必ずraw保存・確認
4. **trait境界** - core/adapters/orchestrationを分離
