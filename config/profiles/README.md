# 002 - Profiles

## Purpose
Environment-specific endpoints and connection settings. These define where REST/WS/auth calls go.

## Format
- TOML files keyed by environment (e.g., prod.toml, staging.toml).
- Fields typically include REST/WS base URLs and environment name.

## Impact Scope
- A profile change affects all API calls (auth, trade, account, ws).
- Incorrect URLs will break connectivity and can cause order routing mistakes.

## Writing Rules
- Do not place secrets here.
- Keep one profile per environment.
- When adding new endpoints, update only the relevant profile file.
