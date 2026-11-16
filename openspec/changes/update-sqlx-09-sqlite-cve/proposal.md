# Change: sqlx 0.9 (dbd + sensei) and rusqlite on libsqlite3-sys ≥ 0.39 — clear RUSTSEC-2026-0459 (#446)

## Why
`cargo audit` reports RUSTSEC-2026-0459 (CVE-2026-119375, medium): bundled SQLite in
`libsqlite3-sys` 0.30.1 ships in senseid via `rusqlite` 0.32 (transcript readers for Zed, VS Code,
Copilot CLI, opencode) and via `dbd-core` → `sqlx` 0.8. `links = "sqlite3"` allows one version, so
both paths move together. v0.22.0/0.22.1 shipped under a user-approved exception
(`.cargo/audit.toml`, `docs/backlog.md`).

## What Changes
- dbd (sibling repo): `sqlx` 0.9, released (a new tag).
- sensei: `sqlx-core`/`sqlx-postgres` 0.9, the new `dbd-core` tag, `rusqlite` on `libsqlite3-sys` ≥ 0.39.
- The audit ignore for RUSTSEC-2026-0459 and its backlog exception are removed.

## Impact
- Specs: `supply-chain`. Code: workspace `Cargo.toml`/`Cargo.lock`, sqlx call sites (0.9 API
  changes), transcript readers (rusqlite), dbd (its own repo and release).
