PolyMarketBot - Rustベースの arb bot

## 開発原則
- **決定論・可観測性**: 同一データ・設定→同一結果
- **責任分離**: 1プロセス1責任、複数プロセス同時稼働設計
- **設定外出し**: ハードコード禁止、設定ファイル必須
- **バックアップ必須**: .env変更時は必ずバックアップ
- **共通化**: 重複情報は適切に共通化
- **ログ整理**: 肥大化防止、適宜修正削除

## プロジェクト概要
⸻

リポジトリ構成（主要）
⸻



## 開発時のルール
- docs作成時には作成順序がわかるように冒頭に通し番号を付ける



# Repository Guidelines

## Project Structure & Module Organization


## Build, Test, and Development Commands


## Coding Style & Naming Conventions


## Testing Guidelines


## Commit & Pull Request Guidelines
Commit history favors descriptive, sentence-style summaries (often in Japanese) that explain motivation and impact. Mirror that clarity: lead with the feature or fix, then note strategy adjustments or migration needs. For pull requests, provide a concise overview, enumerate test commands executed, attach relevant logs or screenshots, and link tracking issues or implementation guides. Highlight schema changes and configuration impacts so reviewers can coordinate database updates.

## Environment & Security Notes
Store secrets in local `.env` files only, never in version control. When sharing logs, redact order IDs and account-specific data.

## Code Responsibility & Abstraction Rules
- **Single Responsibility Principle**: Keep each module or function focused on a single concern; avoid mixing unrelated responsibilities.
- **No Hardcoding or Hidden Dependencies**: Do not embed runtime values directly in code. Surface all configuration through external files such as `configs/` or `.env`.
- **Business-Agnostic Naming & Logic**: Choose names and abstractions that are independent of a specific business workflow so modules remain reusable.
- **Configuration Injection**: Inject parameters from external sources so behavior can change without modifying source files.
- **Explicit Boundaries**: Clearly separate domains—core logic, adapters (APIs/DB), orchestration, presentation—and express boundaries via traits or interfaces to keep coupling low.

## Conventions for Creating Design Documents During Development
When I conduct development work, I always prepare several types of design documents before implementation. Each document has a clear purpose and role, and they are organized and managed according to the following rules. There are four types of documents that serve as references during implementation:
#### Planning Document (dev/planning)
Describes the steps toward major goals and defines the overall implementation direction and framework.
#### Implementation Guide (placed in the project root)
Divides the planning document into medium-sized units such as phases and summarizes the implementation policy and key considerations. This document is intended for management purposes and serves as a parent document overseeing all task lists.
#### Task List (dev/impl)
Breaks down the implementation guide into issue-level units and describes specific work details. Each file should be named in a way that clearly indicates which implementation guide it belongs to, for example: dev/impl/impl_guide_120_issueA_feature_kernel_refactor.md. This document lists all related files, functions, and structures comprehensively, defines their names and expected behaviors, identifies areas affected by changes, and determines responses according to their importance. Progress is managed through checklists.

#### Discussion (dev/discussion)
Organizes items that must be confirmed before implementation and is used to request clarification or decisions from me (the project manager) in order to finalize the approach. Each file is created with a sequential number, for example: dev/discussion/discussion_XXX.md. After receiving feedback from me, the agreed content is reflected first in the implementation guide and then in the task list. (The detailed requirements for what to include in a discussion document will be defined separately.)

### Implementation Guide Details
Implementation guides are created in the project root using the format: impl_guide_<serial_number>.md (e.g., impl_guide_120.md). This document is intended to manage all related task lists and focuses on the following items rather than detailed explanations: Checklists, Key points and precautions, Implementation policy, Verification methods, Goals and objectives, and Task breakdown.

### Task List
Task lists are managed on an issue-by-issue basis. Even if there are multiple task lists, each must comprehensively list all related files, functions, and structures.At this stage, define the names and expected behaviors clearly, identify all areas affected by modifications, and organize responses according to importance. Progress is tracked using checklists.



## API-related implementation guide
Before implementing any API-related logic, **you must always create a temporary test file to capture the raw API response**. Never attempt to parse, map, or structure the data before verifying what the real (raw) response looks like.

---

### Rules

1. **Always fetch the API once and log the full raw response.**
2. **Save the response to a project-local raw directory** (e.g., `src/tests/rawResponses/fee_20250101.json`).
3. **Inspect the saved file manually** to understand the true data structure.
4. **Only after confirming the real data shape**, proceed with parsing or implementing logic.
5. **No exceptions** — this applies to all endpoints and all API fetches.

---

### Example Workflow in This Repo

1. Create a lightweight script under `src/tests/raw/` that calls the SDK or REST client you intend to use.
2. Run it once and inspect the saved file.
3. Only after confirming the raw structure should you implement parsing or business logic.

---

### Goal
The purpose of this rule is to **eliminate wasted debugging time** from incorrect assumptions about API structures. You must always **see the real response first**, then design the parser or data model based on actual data, not documentation or speculation.
