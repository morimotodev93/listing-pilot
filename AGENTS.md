# AGENTS.md — Listing Pilot

This file provides standing instructions for coding agents working in this repository. Follow the user's current task and the repository's architecture decisions. Keep changes focused; do not infer permission for unrelated work.

## Project and environment

- **Product:** Listing Pilot — 「写真から、出品準備完了まで。」, a Windows-first desktop application for preparing ecommerce product listings.
- **Stack:** Tauri 2, Rust stable (MSVC on Windows), React 19, TypeScript, Vite, SQLite through `rusqlite`, and npm.
- **Architecture:** A modular monolith: one Rust crate in `src-tauri/`, feature-organized React in `src/`.
- **Canonical package and dependency locks:** `package-lock.json` and `src-tauri/Cargo.lock`. Do not switch package managers or upgrade dependencies without a task-specific reason.
- **App identifier:** `com.listingpilot.app`. Avoid modifying it or changing data locations without an explicit migration plan.

## Read before editing

1. Check `git status --short` and inspect the relevant current files, tests, and nearby changes. Preserve all pre-existing work, including untracked files.
2. Read `README.md`, `docs/architecture.md`, and the applicable documents in `docs/decisions/` when making architectural or data-handling changes.
3. Identify the smallest implementation and test change that satisfies the task. Ask before making an irreversible or unexpectedly broad change when possible.
4. Do not assume that older phase notes describe the current implementation; inspect the code.

## Architecture boundaries

- **React:** UI and view state live under `src/features/`; components access backend functionality via `src/lib/api/`, not direct Tauri `invoke()` calls. Avoid introducing a generic web adapter until needed.
- **Tauri commands:** Keep `src-tauri/src/commands/` thin. They validate or coordinate IPC boundaries and delegate work to services.
- **Rust domain/services/infrastructure:** Use `domain/` for models and domain constraints, `services/` for workflows, and `infrastructure/db/` and `infrastructure/media_storage.rs` for persistence and filesystem implementation.
- **SQLite and filesystem access:** Keep on the Rust side. Do not expose arbitrary SQL, arbitrary-path file access, or broad filesystem access to the frontend.
- **Pragmatism:** Prefer existing modules, explicit types, and small functions. Do not introduce a framework, repository traits, dependency-injection container, event bus, connection pool, or extra crates merely for hypothetical future needs.
- **Concurrency:** Avoid holding the SQLite `Mutex` during image copying, decoding, hashing, or other slow filesystem operations. Use blocking workers for blocking Rust work invoked by async commands.

## User data is not disposable

- The app stores its SQLite database and managed media beneath Tauri's `app_local_data_dir()`. Development and installed builds may share an app identifier and data location; **never treat that location as test data**.
- Never delete, reset, silently rebuild, or overwrite the user's SQLite database or original images to fix an error. Do not silently fall back to an in-memory database after initialization failure.
- Treat SQLite records and managed media files as one logical dataset. Backups and restore procedures must account for **both** the database and `media/`, with the app stopped.
- Use numbered, forward-only SQLite migrations; preserve existing migration files and their behavior. New migrations should be transactional and covered by tests for existing-data preservation and failure cases.
- Preserve source images unchanged. Store managed originals under controlled UUID-derived paths; validate file content, size, dimensions, and paths in Rust. Keep per-product duplicate detection and database uniqueness constraints intact.
- Respect the media OS lock and in-process import guard. For startup reconciliation, verify database access first, then acquire the media lock and **re-read current database references while holding it** before deciding which files are unreferenced.
- If database state is unknown, do not assume there are zero references. Prefer retaining or quarantining uncertain files over deleting user data. Do not replace an existing managed original during final placement.
- No destructive delete, bulk reset, or cleanup behavior without an explicit product requirement, clear user-visible semantics, and appropriate tests.

## Security and permissions

- Preserve the restrictive production CSP and separate development CSP (`src-tauri/tauri.conf.json`); do not add `unsafe-eval` or broaden permissions for convenience.
- Keep the Asset Protocol scoped to the intended managed originals, not staging, quarantine, the database, or arbitrary paths.
- Add only the Tauri capabilities and plugin permissions necessary for a feature. Prefer the existing Rust-native file dialog approach for image import.
- Validate untrusted inputs at the Rust boundary, use parameterized SQL, and reject unsafe paths and links/reparse points where relevant.
- Avoid exposing sensitive local paths, database internals, or secrets in UI errors, logs, fixtures, and screenshots.

## Local tests and fixtures

- Run automated tests against dedicated temporary SQLite databases and media directories, never the real app's LocalData. Prefer deterministic tests over timing-dependent sleeps; cover failure compensation and restart persistence when relevant.
- `test-fixtures/` is **local-only and ignored by Git**. It may contain generated sample products and images for manual demonstrations. Do not assume fixtures are shipped or available in CI, and do not add code that silently imports `products.json` unless explicitly requested.
- Keep `.verification/` and other local smoke-test artifacts out of version control. Do not modify or delete existing fixtures, screenshots, or verification outputs without permission.
- For manual UI checks use `npm run tauri dev`; `npm run dev` starts only the Vite frontend and cannot exercise normal Rust IPC on its own.

## Verification and reporting

Run the checks relevant to the change, normally from the repository root:

```powershell
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

- For relevant Tauri integration, security, installer, or native-code changes, also verify the desktop app and use `npm run tauri build` when a release/package check is warranted.
- If Vite fails with `spawn EPERM` in a constrained sandbox, investigate the execution environment and rerun with the minimum necessary approved permissions before changing application code or dependencies.
- Re-run affected checks after the **last** source edit. Do not claim that an installer, another PC, power-loss recovery, or real WebView2 flow was tested unless it actually was.
- Update `docs/verification.md` with meaningful verification evidence and limitations; update architecture documentation or an ADR when a lasting design decision changes. Avoid documentation churn for trivial edits.
- Report in Japanese: what changed, why, tests and exact outcomes, remaining risks or untested scenarios, and any user action needed. Distinguish observed results from assumptions.

## Git and scope discipline

- **Never commit, push, switch branches, rewrite history, or publish a release unless explicitly instructed.**
- Never discard pre-existing local changes. Do not use destructive cleanup commands (`git reset --hard`, `git clean -fd`, database deletion, or equivalent) without explicit authorization.
- Avoid unrelated formatting, lockfile changes, icon regeneration, environment modifications, or dependency upgrades.
- Before concluding, review `git status --short` and `git diff --check`; remember that untracked files do not appear in ordinary `git diff` output.

## Reference documents

- `README.md` — setup, commands, implemented features, and storage guidance.
- `docs/architecture.md` — current code organization and integration flows.
- `docs/data-model.md` — database structures and data behavior.
- `docs/decisions/` — accepted architectural and security decisions.
- `docs/verification.md` — verification history and known gaps.
