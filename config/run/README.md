# 003 - Run Configs

## Purpose
Runtime selector files that compose app/profile/markets/plan (and future universe/risk/strategy) into one executable configuration.

## Format
- TOML files with include paths (relative to the run file).
- `mode.trading` controls DryRun/Paper/Live.
- `mode.requires_env` is a safety gate for Live (e.g., ALLOW_LIVE_TRADING=YES).

## Impact Scope
- Switching the run file changes the entire runtime behavior without code changes.
- Live gating prevents accidental production execution.
- [execution] は必須（欠落時は起動エラー）。
- 確認待ちのタイムアウトは `execution.confirm_timeout_secs` で調整する。

## Writing Rules
- Use relative paths to keep configs portable.
- Do not reference .env here; it is read separately for secrets.
- Any change to run configs should be reviewed like a deploy change.
