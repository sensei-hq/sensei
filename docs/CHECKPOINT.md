# Checkpoint

**Slice:** Repository sharing — **DONE, verified live.**
`d27ffa1f` · `75b00683` · `849fa070` · `1182ab4b` · `b0b68f62` · `76bc040b` ·
`7b0f53ce` · `e4232ccb` · `9d4e8441`

## Live state

```
corpus   private  organization (MANDATED)  -> not_subscribed
dbd      public   user  elected            -> SYNCING
torii    public   user  elected            -> SYNCING
gateway
sensei   public   user                     -> not_elected_user   (flip in the UI)
```

Full cycle proven: `repo` scope granted (GitHub reports `read:org, repo,
user:email` and sees the **private** `sensei-hq/corpus`) → capture → two-axis
verdict → election over HTTP → push → 132 rows, 0 pending, idempotent.

## Review backlog, worked

**Fixed** (`9d4e8441`) — PostgREST caps every read at 1000 rows
(`PGRST_DB_MAX_ROWS=1000`, read off the running instance). Three reads were
unbounded; the **ingest** one refused legitimately-permitted repos as
`not_permitted`. Reads now page (stopping on a *short* page); the ingest filters
by the batch's keys instead. Also: `drop view` discards ACLs and the DDL had no
`grant` — the live ACL was `postgres` + `service_role` only.

**Fixed** (`e4232ccb`) — the Sharing screen. The election had a write path and no
way to reach it.

**False, corrected** — `configurable_by_me` grants `lead` (it grants `admin`
alone); `uptimeSeconds` is wrong (it reports 1259s for a 1238s process — I had
compared against a process that did not hold port 7744).

**Deferred, with rationale** — a 404 leaves a stale visibility standing, bounded
at 30 days by `forge_visibility_stale`. Not fixed by clearing the capture:
that would trigger the new self-heal every 60s for a deleted repo, trading
bounded staleness for an unbounded poison pill.

**New, measured** — `GET /health` takes **13 seconds**; `bootstrap::check` probes
binaries synchronously on every call.

## The recurring lesson, third time

**The fake is not the real thing.** The pagination tests passed immediately —
`fakeDojoDb` returned 2350 rows, green against code that did not paginate. It now
enforces `PGRST_MAX_ROWS` and supports `.range()`. Same shape as the invented
`elected_by` column and the stub that keyed on call-count rather than causation.

## Next

Nothing blocking in this slice. Candidates: **phase 2 of
`dojo-auth-provisioning`** (`claimed_at`, `seat_allocations` — the billing terms
already sit COMMENTED at their precedence positions in the view); the **GitHub
App migration** (backlog, with incremental consent as the cheaper interim); **D3
governance pull**.

**Gates:** daemon 2492 exit 0 · clippy 0 · fmt 0 · dōjō 1505 · check 0/0.
