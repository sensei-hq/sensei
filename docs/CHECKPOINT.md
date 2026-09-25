# Checkpoint

**Slice:** production readiness (issue #130 phase `build` → next is assessment)

## State

**Still 0.9.1 — NOT bumped.** The release checklist wants a green e2e and it is
23 red. `make bump v=minor` publishes (tag → `release.yml` → homebrew tap +
marketplace subtrees) and carries **38 unpushed commits**, so it waits on a
decision, not on more work.

| gate | result |
|---|---|
| senseid suite | **3,318 pass / 0 fail** |
| clippy `-D warnings`, fmt | clean |
| app unit / bootstrap | 1,698 / 174 pass |
| **Tauri e2e** | **107 pass / 23 fail / 20 skip** |
| `make install` | Sensei.app 08:10, daemon 0.9.1 healthy |

## What this session established

- **E2E diagnosed, and the first diagnosis was wrong.** It is NOT cold-database
  timing: `start_server` binds BEFORE touching the DB, on purpose, and falls
  back to a degraded router that self-heals — so a slow DDL cannot stop the port
  opening. The daemon never reached `start_server` at all (it wrote neither
  `senseid.log` nor `serve.pid`). Candidate: the already-running PID guard in
  `main.rs`, whose message goes to the parent's stderr — which was
  `stdio: 'ignore'`. `globalSetup` now captures it (`790f90b7`).
  See readiness **B3** (pre-bind exits skip `write_startup_error`, which has
  three callers and all are post-bind) and **B4** (`Full`/`Degraded` is two
  states for three situations — provisioning is not a fault).
- **Root add/remove journey passes** (`setup-wizard.spec.ts`).
- **Dōjō:** fixed a real blocker — vite binds IPv6, the daemon dials IPv4, so
  `dojo_sync` could never connect (`--host 127.0.0.1`, now `last_ok=t`). But
  **the sync has never sent a row**: membership stuck at `authenticating`,
  outbox `held` since 2026-08-29.
- **Token metrics exist** (5 of them) but `metered_cost` is never non-zero and
  `Tokens per day` is 99.9996% cache reads.

## Next command

```
# pick one, then continue:
make bump v=minor          # publishes 0.10.0 + 38 commits
# or triage the 23 e2e failures first — 4 configure-assistants, 4 boot-flow,
# 3 instruments-t2-slices, 2 each daemon-verification/atlas
```

## New this session

- `CHANGELOG.md` — starts at 0.10.0; the 67 prior tags are not reconstructed.
- `docs/production-readiness.md` — **20 measured items**, each with the query
  beside the number. Top five: **A1** `adopt_node_by_identity` uses `fetch_one`
  so a moved row fails the whole file (44 today) · **C1** `public.logs` is 25 GB
  of a 37 GB database · **D1** CI runs no Rust tests, clippy or fmt · **E1**
  dōjō has never synced · **A2** cost is never measured.

## Known-broken

- Indexing never fully completed: 380 folders `discovered`, 12 `indexing`.
- Deps behind, untouched: rokkit 1.4.0 (app) / 1.3.4 (dojo) vs **1.6.0**;
  kavach 1.1.3 vs **1.2.0** (pinned exact, needs a manifest edit).
- Supabase is up (33 containers) and dojo dev is on :5173 — both will need
  restarting next session.
- SQL/Kotlin/Swift unflipped, so `crate::languages` still ships.
