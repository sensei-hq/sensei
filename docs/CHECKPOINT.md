# Checkpoint

**Slice:** Dōjō metric push (`docs/spec/dojo/daemon-sync.md`) — **shipped and
live-verified**, then adversarially reviewed and partly hardened.

## The cycle works end to end

`register → plan → push → watermark`, verified against the live databases:
132 rows in `dojo.repository_metrics`, 132 marked `shared_at`, queue drained to 0,
re-pushes updating in place. `sync_state` holds `dojo_sync_plan/default synced`
and `repository_metric/github.com/sensei-hq/dbd/push synced`.

Four bugs were found by RUNNING it, not by any test (`41fefe73`). The worst: the
registry returned `personas.label` where the Keychain slot was the sign-in hint
string, so the persona was skipped while the tick reported success. Fixed with
`personas.session_slot`, which records what the sign-in actually used —
**`label` is NOT the slot, a sign-in rewrites it to the verified login.**

## Adversarial review — 4 of 5 reviewers reported

Fixed (`7341101f`, `2dc0d089`, and this commit): two ingest CRITICALs — a
repo_key under two tenants 500'd every push forever, and the upsert key matched
4 of 7 columns so rows merged silently (6 groups / 34 rows on the live DB).
Plus `nulls not distinct` on `dojo.repository_metrics`, which makes claim C5
true rather than merely recorded; the label/slot correction in all four
surviving places; the claims ledger re-verified; §6's "two schema changes"
corrected to five; D3/D4/D5/D8 marked deferred or superseded.

**Still open, severity order:**

1. **CRITICAL** — `tick`'s whole body can be replaced with `Ok(())` and both its
   tests pass. No real coverage of: push error, partial acceptance, unparseable
   `tenant_id`, empty allow-list, per-persona isolation. Needs an injectable
   transport; `crates/senseid` has no HTTP-mock dev-dep today.
2. **HIGH** — plan allow-list still filtered AFTER the SQL LIMIT. Same class as
   the scope bug already fixed: 218 of 500 slots go to never-mappable repos.
3. **HIGH** — `a_pushable_row_carries_every_field_the_dojo_needs` is vacuous for
   5 of 7 fields: the fixture value IS the constant a broken projection emits.
4. **HIGH** — the `scopes` parameter is untested; every call site passes both
   enum values, so the filter is a tautology.
5. **HIGH** — `POST /v1/you/metrics` has no test; all six siblings do.
6. **HIGH** — `unknown_repository` untested; delete the reject and rows are
   silently dropped AND watermarked.
7. **MEDIUM** — a 500-row batch is ≈2001 sequential PostgREST subrequests in one
   Worker invocation. The live run moved 132 (≈529) — a quarter of the load.

The security reviewer had not reported when this was written.

## Next command

```
rg -n 'fn tick' crates/senseid/src/tasks/dojo_sync.rs      # finding 1
```

## Gates (green)

daemon 2463 tests exit 0 · clippy 0 · fmt 0 · dōjō 1429 tests · check 0/0.

## Known-broken / carry-forward

- **`~/.sensei/config.json` `dojo_url` points at `http://127.0.0.1:5173`** for the
  live test. Production value backed up at `/tmp/sensei-config-backup.json`.
- `dbd` is `shared`; `dojo_sync` cadence is 60s. Both are test settings.
- Debug binaries are installed in the brew prefix (`make install-debug`).
- **kavach 1.1.1 publish + repin** — `node_modules/kavach` is patched locally
  with `040d34c`. *User owns this.*
- D3 (governance pull), D5's change-guard and D8's public/private default are
  deferred, now recorded in §9. D8 is *unimplementable* today — no
  forge-visibility column exists.
- `dbd diff --scope default` is never clean (dbd normalises `time` and unnamed
  CHECKs). Filed in `docs/backlog.md`.
