# Checkpoint

**Slice:** Schedules (`docs/spec/daemon/schedules.md`) — **all 5 steps done**.
Now into `docs/spec/dojo/daemon-sync.md`.

## Done

| # | what | commit |
|---|---|---|
| 1–3 | table + seed + agreement test; `should_run` predicate; one generic ticker | `78cd7808` `1dc4dd6c` `209349d6` |
| 4 | `GET`/`PATCH /api/tasks/scheduled` read and edit the table | `2ed0d4fd` |
| — | persona registry, D2 rename, D7 enum value | `eab5671d` |
| — | `live_session` — one implementation of "get me a working credential" | `083aacc7` |
| — | dōjō sends `tenant_id` on both `/v1/you` endpoints | `1641069a` |
| 5 | `dojo_sync` — a scheduled task with no bespoke worker | `8bea1832` |
| — | app shows WHEN each worker runs; `avg` column retired | `45b56a8f` |

## Two spec claims were false and are corrected in place

- **§3's `sensei.dojo_personas`** was never built. `sensei.personas` already is
  the registry (`label` = the Keychain slot; `verified_at` = signed in), the
  dōjō URL is global, and `sync_state` carries the watermark. *(User approved.)*
- **§1's "`unpushed_metric_rows` is the one production push path"** — it has no
  production caller at all, and no dōjō endpoint receives metrics.

## Next

**The metric push is a slice of its own** — that is the honest state.
`dojo_sync` today establishes identity (which tenant) and entitlement (what may
sync) and logs that it pushed nothing. It does not pretend otherwise.

```
rg -n 'unpushed_metric_rows' crates/senseid/src/db/pg_store/sync.rs
```

Then D3 (per-repo governance pull), then phase 2 of `dojo-auth-provisioning.md`.

## Gates (green at `8bea1832`)

`cargo test -p senseid` 2454 · clippy 0 · fmt 0 · dōjō 1421 tests, `check` 0/0.
Every new invariant mutation-probed.

## Known not done

- **kavach 1.1.1 publish + repin** — `node_modules/kavach` is patched locally
  with `040d34c`. *User owns this.*
- `dbd diff --scope default` is never clean: dbd normalises `time` and unnamed
  CHECKs so `sensei.schedules` always shows a phantom type change. Not fixable
  from the DDL — both spellings diff. Real drift is still readable, but only by
  ignoring those four lines.
- **Live `sensei` DB is UPDATED** (backup `database/backup/backup-20260828-084938.dump`,
  1.6G). Rename was lossless — 67 repositories, 0 carried `dojo_id`. 12 schedule
  rows loaded through the real `staging.import_schedules()`; table and
  `SCHEDULABLE` agree both directions. The RUNNING daemon is still the old
  binary and ignores `sensei.schedules` until `make install-service`.
- `make clean` run 2026-08-28: reclaimed 40Gi (target/ was 40G), 59Gi now free.
  **`target/` is empty — the next build is a full rebuild.** Backups are pruned
  only at >7 days, so all 6 dumps (~9G) survive, including the live-DB rollback.
