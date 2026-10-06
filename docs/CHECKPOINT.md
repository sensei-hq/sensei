# Checkpoint

**Slice:** Observatory diagrams — data → derivation → screens. Lanes 1 (data) and
2 (derivation) are DONE for the architecture set. Nothing is blocked on data any
more; what is left is screens, plus two foundations re-prioritised on 2026-10-05
(persistence fold-in, resolution attribution).

## Next — autopilot queue (in order)

Sequence re-set 2026-10-05. Each item sits where nothing earlier depends on
anything later; the reasoning is below the list, not implied by it.

    [x] #215 registering a root enqueues a scan             91809876
    [x] #216 watcher batch needs no database                91809876
    [x] #224 git history scanner                            964a99b6
    [x] #222 derivation: SCC + layering                     83c9f9ab
    [x] #223 derivation: Zones (+ the matrix needs none)    8ec2fa6c
    [x] #217 governance seed drift — ponytail is opt-in     6cbba6ee

    -- A. persistence (settle the pattern before 30 more land on it) --
    [x] #161 8+-arg writers → typed rows   8fe58786 bfea831e a1e45890
        10 row types, 0 functions left at 8+. 41 remain in the 5-7 band,
        left open deliberately — that tail is multi-key lookups, not
        a producer's struct spelled out, so it is a different risk.
    [ ] #227 fold the 30 queries into PgStore + a grep gate  ← NEXT

    -- B. indexing truth (the instrument before the fixes) --
    [ ] #235 resolution by (language, kind, outcome)
    [ ] #231 fqn segment 3 is a SYMBOL for some adapters

    -- C. screens --
    [ ] #232 Layers + Cycles screens — `up` is now POPULATED (2e898607)
    [ ] #233 diagram read cost — decide WITH a real screen
    [ ] #219 World · #220 Neighbourhood · #221 Schema

    -- D. consent + dōjō --
    [ ] #218 transcript ingestion opt-in
    [ ] #236 dōjō → kavach data routes
    [ ] #225 dōjō team view (needs the sync-shape decision)

    -- deferred by decision --
    [ ] #237 C#/JS indexing coverage — after #235, so the delta is readable
    [ ] #230 declared layering · #234 pack adoption UI — both need a product call

Per slice: TDD, mutation-probe each test, full suite + clippy + fmt +
`scripts/check-sql-against-schema.py`, then close the issue with evidence.

## Why that order

- **#161 before #227.** #227 adds ~30 `PgStore` methods; #161 is converting that
  family off positional arguments. The reverse order writes 30 functions in a
  style already decided against — and `upsert_project_metric_repo` (10 positional
  args) is on #161's list AND in #227's population.
- **#235 before #231 and #237.** A corpus-wide resolution percentage mixes "never
  indexed" with "indexed and the resolver missed". Changing either without the
  breakdown means changing it blind, and arguing afterwards about what moved.
- **#232 before #233.** The right caching answer (matview · refresh-on-barrier ·
  daemon cache) depends on how a real screen consumes the payload.

## Open questions

- #230 is NO LONGER BLOCKING (2e898607). "Which calls break the downward flow"
  is answered by a cycle's back edges — 25 of them on project `sensei`, led by
  `senseid/db → senseid/tasks ×149`. A declared layering is now only for the
  narrower "does the measured structure match the one you intended".
- #234 nothing lets a developer ADOPT a seeded rule pack. DECIDED 2026-10-05:
  the MCP tool pair (`list_available_packs` / `adopt_pack`) writes it first;
  CLI and app screen follow if they earn it.
- `sensei.assistants` is an EMPTY table with no reader or writer (4 tree
  references, all prose). #218's consent should use a `sensei.config` key; the
  table itself wants a separate give-it-a-writer-or-drop-it call.

## Known broken

- **#235 imports resolve at 2.6%** — 212,270 unplaced-with-a-name against 6,622
  placed, while 68,287 `lib·` nodes exist and calls/references DO reach them.
- **#237 ~11,100 tracked files have no nodes**: C# 52.3%, java 79.9%, js 78.4%,
  py 74.2%. Cause NOT established — check whether those 9 folders ever finished
  before suspecting the adapter.
- **#231 an fqn's segment 3 is a SYMBOL for some adapters, not a module.** 13.7%
  of `sensei`'s module dependencies and 78% of the largest client project's
  point at a unit that owns no file. Counted as `coverage.unknownUnit`; the
  shipped Structure diagram has the same hole (31 of 213), uncounted.
- `create_memory` has NO production caller (all 17 sites are tests), and
  `sensei.memories` has three writers. Now `#[cfg(test)]`; which one production
  uses is unanswered.
- `project_health` and `unused_tools` have NEVER produced a metric row despite
  being effective since 2026-08-09. `cost` last computed 2026-09-20 while every
  other family ran 2026-10-05, with a subscription configured.
- **dbd#41 blocks `dbd reconcile` on any database `dbd deploy` created.** It
  emits a false `ALTER TYPE` for a column whose extension type lives off the
  search_path — the configuration `design.yaml` asks for. Workaround: apply the
  one changed file with `psql -f`, or drop and redeploy `sensei_test`.
- **App e2e 114/21/17 skipped** — the known red gate (#187). Sampled causes are
  harness defects, not this slice.
- `--features senseid/embedded-llama-cpp` will not compile (#203). `EMBED=0`
  works on every make target and is how the live daemon was built.
- 70+ commits UNPUSHED on `develop`. Otherwise green: senseid 3270/0, clippy
  `-D warnings` + fmt, dbd doctor, SQL statements plan against a fresh deploy
  (and that planner now runs in CI).
- Two unattributed single-run failures, both passing on rerun and in isolation:
  `audit_repairs_nested_standalone` and `prune_empty_projects_…`. The latter uses
  fixed project names and runs a GLOBAL prune, which is the #183 class. The one
  race I could name — two tests fighting over the `cost.subscription` config key
  — is fixed with a mutex.
