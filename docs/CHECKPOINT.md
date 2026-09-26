# Checkpoint

**Slice:** production readiness (issue #130 phase `build` → next is assessment)

## State

**v0.10.1 is released and installable.** `sensei --version` → 0.10.1; `brew
fetch` verifies both formula and cask against the published assets.

| gate | result |
|---|---|
| senseid suite | **3,303 pass / 0 fail** (also green on a CI-shaped fresh schema, 233s) |
| workspace rest | 358 pass / 0 fail |
| clippy `-D warnings`, fmt | clean |
| Coverage workflow | **green** |
| Rust CI `lint` | **green** (now includes `make check-ddl-grants`) |
| Rust CI `test` | `Install dbd` ✓ `Provision sensei_test` ✓ — suite was still running at checkpoint |
| **Tauri e2e** | **107 pass / 23 fail / 20 skip** — unchanged, still the open gap |

## What happened after the bump

v0.10.0 tagged and **published nothing**: its aarch64-linux leg failed, `release`
needs every leg, so assets, Release and `update-tap` were all skipped — while
`make bump` had already pushed the tap naming 0.10.0 with
`REPLACE_WITH_*_SHA256`. `brew install` was broken. Four fixes, then v0.10.1:

- **aarch64-linux builds natively** (`ubuntu-24.04-arm`). Under `cross`,
  `pg_query`'s bindgen ran a clang that could not find its own `stdbool.h`;
  `pg_query` comes via `dbd-core`, so it is not optional. `cross` is gone.
  Matrix is no longer fail-fast — one failing leg used to cancel and silence the
  other three, so each attempt cost a tag to learn one fact.
- **A fresh Postgres could not provision at all** (#196, closed). Twelve
  `sensei`/`activity` files grant to Supabase's `anon`/`authenticated`/
  `service_role` and nothing created them, so the first GRANT aborted the deploy.
  Second first-install blocker found today, same way as #190 — by provisioning a
  genuinely fresh database. Invisible locally because this machine has the roles.
- **Rust CI had two errors of mine:** the package is `dbd-cli` (the *binary* is
  `dbd`), and `dbd deploy` with no `--scope` applies the full set, not the
  `default` scope the daemon deploys.
- **Pure coverage was running DB tests** — a libtest filter is a substring, so
  `libraries::` also selected `tasks::handlers::libraries::`. 33 tests pulled in,
  7 failing on `pool timed out`.

## Next command

```
# 1. confirm the Rust CI test job went green (it was mid-suite):
gh run list --workflow=rust.yml --limit 1
# 2. then the e2e, which is the last red gate:
cd app && bun run test:e2e
```

## Open questions

- **#198 blocks every future release from finishing green.** `TAP_GITHUB_TOKEN`
  is unset/expired so `update-tap` cannot write the tap — 0.10.1's SHAs were
  filled BY HAND and that does not generalise. Separately `deploy-dojo` has no
  `DOJO_DATABASE_URL` and correctly refuses; that is a **Postgres wire-protocol
  URL for applying DDL**, not a stand-in for the Cloudflare Worker's
  `PUBLIC_SUPABASE_URL` (PostgREST/Auth over HTTPS) — the two are different
  layers. The real question is whether that job should exist at all, since the
  dōjō schema has been pushed by hand since `dojo-mind` was removed. Both need a
  decision, not code.
- **#197** — four different dbd versions pinned (release.yml v0.12.0, bootstrap
  v0.14.0, senseid v0.15.0, rust.yml v0.17.0), and release.yml violates the
  invariant it documents. Two `dbd-core` copies compile, so `pg_query` builds
  twice.
- **#195** — vitest path traversal (dojo on 3.2.7, app on 4.1.5, patched 4.1.11)
  and glib unsoundness via Tauri.

## Known-broken

- E2E is 23 red and outside the static gates (#187).
- `java_heritage_over_the_real_tree_…` is permanently vacuous: Java is v2's now,
  so v1's `DetectionOnly` returns no `fqn_output` and every file is skipped. Now
  documented as such; the property wants re-homing in v2's corpus tests.
- Dōjō has never synced a row — membership `authenticating` (#184–#186).
- Indexing never fully completed: 380 folders `discovered`, 12 `indexing`.
- Deps behind: rokkit 1.4.0 (app) / 1.3.4 (dojo) vs 1.6.0; kavach 1.1.3 vs 1.2.0.
- SQL/Kotlin/Swift unflipped, so `crate::languages` still ships.
