# Change: Clear RUSTSEC-2026-0459 — rusqlite on libsqlite3-sys 0.40, and no SQLite through dbd (#446)

## Why
`cargo audit` reports RUSTSEC-2026-0459 (CVE-2026-119375, medium): bundled SQLite in
`libsqlite3-sys` 0.30.1 ships in senseid via `rusqlite` 0.32 (transcript readers for Zed, VS Code,
Copilot CLI, opencode) and via `dbd-core` → `sqlx` 0.8 → `sqlx-sqlite`. `links = "sqlite3"` allows
one version, so both paths move together. v0.22.0/0.22.1 shipped under a user-approved exception
(`.cargo/audit.toml`, `docs/backlog.md`).

## Plan change (2026-11-16, found at build)
The first plan moved sqlx to 0.9 in dbd and sensei. sqlx-sqlite 0.9.0 still pins
`libsqlite3-sys >=0.30.1, <0.38.0`, below the fix (≥ 0.39), so sqlx 0.9 cannot clear the advisory.
sensei never opens SQLite through dbd (it uses dbd's Postgres adapter, parser, schema model and
deploy), so the dbd path is removed instead: `dbd-core` without its default `sqlite` feature.

## What Changes
- sensei: `dbd-core` with `default-features = false, features = ["postgres", "deploy"]`; `rusqlite`
  0.43 (`libsqlite3-sys` 0.40).
- The audit ignore for RUSTSEC-2026-0459 and its backlog exception are removed.
- No dbd change or release. sqlx 0.9 is no longer part of this fix (an ordinary upgrade, later).

## Impact
- Specs: `supply-chain`. Code: workspace `Cargo.toml`/`Cargo.lock`, `crates/senseid/Cargo.toml`,
  transcript readers (rusqlite 0.32 → 0.43 API), `.cargo/audit.toml`, `docs/backlog.md`.
