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

## Adversarial review — all 5 reviewers reported, all findings fixed

Five independent reviewers over `eab5671d^..HEAD`. Every CRITICAL/HIGH/MEDIUM is
fixed and mutation-probed; commits `7341101f`, `83fcabc8`, `12e325d0`,
`2917ee21`, `fd4b719f`.

**Security (2)** — a live Supabase refresh token was captured from an
unprivileged `ps`: `security -w <secret>` puts it in `argv`, and this slice made
the write recur per cadence. Now fed over stdin through one shared helper. And
`PATCH /api/repositories` was unauthenticated under `allow_origin(Any)`, so any
web page could flip the sharing gate and read the repo inventory; now guarded by
`Origin` (a missing one is allowed — the CLI/MCP send none).

**Correctness (4)** — a repo_key under two tenants 500'd every push forever; the
upsert key matched 4 of 7 columns so rows merged silently; one refused row
livelocked the whole 500-row window; the dōjō unique key lacked
`NULLS NOT DISTINCT` so it fired for nothing we push (claim C5 was recorded
CONFIRMED on a check that could not establish it).

**Reliability (5)** — the plan allow-list joined the scope filter in SQL, before
the LIMIT; four discarded errors restored (rotated-token write, clear-on-rejection,
`mark_sync_error` overwriting the real error, `unwrap_or(0)` printing "0 held
back"); `tick` no longer reports success when every persona failed.

**Ingest hardening (3)** — the reject reason was a cross-tenant existence oracle,
now one reason; the body is bounded before parsing, not after; the resolve reads
are batched O(rows) → O(1) (≈2001 sequential Worker subrequests at 500 rows).

**Tests** — the cycle had none that could fail: `tick`'s body could be `Ok(())`
with both tests green. `user_plane::UserPlane` is now injectable and there are
nine tests asserting database state. Also fixed: a vacuous field-projection test
(fixture value == the constant a broken projection emits), a tautological
`scopes` filter, a `PLAN_ENTITY` test comparing a const to its own literal, and
two missing test files. `fakeDojoDb` learned `.schema()` and read-counting,
because without them the assertions were vacuous.

## Re-verified live on the hardened build

20 re-queued rows pushed, queue drained to 0, `dojo.repository_metrics` unchanged
at 132 — they updated in place, and that now rests on a DB constraint rather than
an app-level check. `Origin: https://evil.example` → 403, `tauri://localhost` →
200, no Origin → 200. Zero processes with a secret in `argv`.

## Next command

Nothing from the review is outstanding. The slice's own remaining work:

```
rg -n 'D3|governance' docs/spec/dojo/daemon-sync.md    # the deferred pull
```

D3 (per-repo governance pull) and D5's change-guard are deferred and recorded in
§9. D8's default is deferred too, but NOT for the reason an earlier version of
this file gave: the forge-visibility column exists
(`dojo.repositories.visibility`, `private | public`, shipped in phase 1). Nothing
POPULATES it, so every row reads `private` — including public repos.

## Gates (green)

daemon 2463 tests exit 0 · clippy 0 · fmt 0 · dōjō 1429 tests · check 0/0.

## Known-broken / carry-forward

- **`~/.sensei/config.json` `dojo_url` points at `http://127.0.0.1:5173`** for the
  live test. Production value backed up at `/tmp/sensei-config-backup.json`.
- `dbd` is `shared`; `dojo_sync` cadence is 60s. Both are test settings.
- Debug binaries are installed in the brew prefix (`make install-debug`).
- ~~kavach publish + repin~~ **DONE.** Published `1.1.3` carries the
  `onSessionSync` hook (7 refs in src + dist + typings, verified from the npm
  tarball); all six `@kavach/*` deps and `kavach` repinned 1.1.0 → 1.1.3 and
  reinstalled clean. No local patch remains.
- D3 (governance pull), D5's change-guard and D8's public/private default are
  deferred, recorded in §9. **Correction:** I twice called D8 "unimplementable —
  no forge-visibility column exists". FALSE: `dojo.repositories.visibility`
  (`private | public`) shipped in phase 1. Nothing POPULATES it, which is a much
  smaller and different gap.
- `dbd diff --scope default` is never clean (dbd normalises `time` and unnamed
  CHECKs). Filed in `docs/backlog.md`.
