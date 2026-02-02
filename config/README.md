# 001 - Config Root

## Purpose
Central entrypoint for runtime configuration. All behavior must be injected from files in this tree (no hardcoding).

## Structure
- app.toml: App-wide paths/logging/db settings.
- profiles/: Environment-specific endpoints and flags.
- markets/: Market metadata snapshots (tick/step/min/max).
- plans/: Order plans (multiple orders, no floats).
- run/: Runtime selector that wires the above together.

## Impact Scope
- Changing any file here alters runtime behavior without code changes.
- Secrets must stay in .env only; this directory must not contain secrets.
- Production execution requires explicit gating in run config (requires_env).

## Writing Rules
- Use explicit types (string/int), avoid float in order-related settings.
- Keep files small and single-responsibility by directory.
- Prefer additive changes to avoid breaking existing runs.
