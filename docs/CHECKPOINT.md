# Checkpoint

**Slice:** Schedules — background work as configuration (`docs/spec/daemon/schedules.md`)

## Done — steps 1–4 of 5

| # | what | commit |
|---|---|---|
| 1 | `sensei.schedules` + `staging.import_schedules` (timestamp guard) + `schedules.jsonl` (11 rows at today's constants) + the code↔table agreement test | `78cd7808` |
| 2 | The pure `should_run` predicate — enabled, due, window (wraps midnight), ISO day mask | `1dc4dd6c` |
| 3 | One generic `ticker`; the twelve workers keep their own `tick()`. Nothing named after a task | `209349d6` |
| 4 | `GET /api/tasks/scheduled` reads the table, not a static registry; `PATCH /api/tasks/scheduled/{name}` edits | `2ed0d4fd` |

Step 4 also fixed a **shipped bug**: `run_scheduled` re-read the schedule every
poll but derived the poll itself once at boot, so a cadence shortened below 60s
never took effect until restart (`Poll::follow`).

## Next — step 5

`dojo_sync` as a schedule row + a `tick()` — the first task with no bespoke
worker. Supersedes **D4** in `docs/spec/dojo/daemon-sync.md`.

```
# start here
rg -n 'dojo_sync' crates/senseid/src/tasks/schedule.rs database/import/staging/schedules.jsonl
```

Then the rest of `daemon-sync.md`: persona registry (`sensei.dojo_personas`),
`live_access_token(persona)`, the user-plane client, `tenant_id` in the API
responses, `repositories.dojo_id` → `tenant_id`, `sync_entity` += `dojo_sync_plan`.

## Gates (green at `2ed0d4fd`)

`cargo test -p senseid` 2441 passed · clippy 0 · fmt 0 · `--test-threads 16` 0/8 flaky.
Four mutation probes confirmed the new tests fail when the behaviour is removed.

## Carry forward

- **`node_modules/kavach` is patched locally** (jerrythomas/kavach `040d34c`).
  Publish `1.1.1` and repin before the dōjō deploys anywhere. *User owns this.*
- `app/src/lib/types.ts` `ScheduledTask` is **not** updated for the new wire
  shape (`enabled`, `interval_secs`, `window`, `days`). The desktop app still
  compiles; it just cannot show or edit the new fields yet.
- Phase 2 of `dojo-auth-provisioning.md` (entitlement) remains unbuilt.
