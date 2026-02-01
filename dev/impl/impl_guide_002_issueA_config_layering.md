# impl_guide_002_issueA - config 分割 + run loader

## Goal
- config を責務ごとに分割し、run.toml の差し替えだけで運用変更できるようにする
- ハードコード排除（URL/銘柄/注文条件/正規化方針は全て config 注入）

## Files（追加/変更）
- ADD: config/app.toml
- ADD: config/profiles/{prod,staging}.toml
- ADD: config/run/{run_live,run_paper}.toml
- ADD: config/plans/plan_{live,paper}.toml
- ADD: config/markets/snapshot.json（初期は空でもよい。後で生成）
- (OPTION) ADD: config/universe/universe_default.toml
- (OPTION) ADD: config/risk/risk_default.toml
- MODIFY: crates/bf_config/src/lib.rs
- (OPTION) MODIFY: config/default.toml（deprecated 表記だけ残す）

## Structures（bf_config）
- struct RunConfig
  - include.app: PathBuf
  - include.profile: PathBuf
  - include.markets_snapshot: PathBuf
  - include.plan: PathBuf
  - include.(universe/strategy/risk): Option<PathBuf>
  - mode.trading: enum { DryRun, Paper, Live }
  - mode.requires_env: Option<String>  # 例: "ALLOW_LIVE_TRADING=YES"
- struct AppConfig（db/log/path）
- struct ProfileConfig（env/url/ws 等）
- struct PlanConfig（IssueCで詳細化）
- struct MarketsSnapshotConfig（パスのみでOK）
- struct RuntimeConfig（上記を統合した最終成果物）

## Functions（bf_config）
- fn load_run_config(path: &Path) -> Result<RuntimeConfig>
  - 仕様:
    - run.toml を起点に参照ファイルを全て読み込む
    - 相対パスは run.toml のあるディレクトリ基準で解決
    - 失敗時は「どのファイル/どのキーが欠けたか」をエラーで明示
    - secrets は読まない（.env は別途）
- fn validate_runtime_config(cfg: &RuntimeConfig) -> Result<()>
  - 仕様:
    - Live の場合のみ requires_env を満たさないとエラー
    - ファイル存在チェック、パスの妥当性チェック

## Checklist
- [ ] config/run/*.toml で「どの設定を使うか」切り替えられる
- [ ] Live 実行は requires_env を満たさないと起動しない
- [ ] URL/銘柄/注文条件がコードに埋まっていない
- [ ] 既存 examples が RuntimeConfig を受け取れる形に更新できる準備ができた
