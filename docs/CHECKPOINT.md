# Checkpoint

**Slice:** Observatory diagrams — data → derivation → screens. Lanes 1 (data) and
2 (derivation) are DONE for the architecture set. Nothing is blocked on data any
more; what is left is screens plus three unwritten endpoints.

## Next — autopilot queue (in order)

Order set 2026-10-03: core computation/indexing → layer semantics → sensei → dōjō.
Issues are the tracker; this is the position in the queue.

    [x] #215 registering a root enqueues a scan             91809876
    [x] #216 watcher batch needs no database                91809876
    [x] #224 git history scanner                            964a99b6
    [x] #222 derivation: SCC + layering                     83c9f9ab
    [x] #223 derivation: Zones (+ the matrix needs none)    8ec2fa6c
    [x] #227 persistence layer — ADR: metric SQL stays     9c7b8794
    [x] #217 governance seed drift — ponytail is opt-in   6cbba6ee
    [ ] #218 transcript ingestion opt-in                ← NEXT
    [ ] #232 Diagrams · Layers + Cycles screens
    [ ] #219 World · #220 Neighbourhood · #221 Schema
    [ ] #225 dōjō team view — needs the sync-shape decision first

Per slice: TDD, mutation-probe each test, full suite + clippy + fmt +
`scripts/check-sql-against-schema.py`, then close the issue with evidence.

## Open questions

- #230 a DECLARED layering — a derived one provably cannot climb, so Layers has
  no violation to show until something stores an intended one. Who authors it,
  where it lives, and whether it belongs to the project or the repository.
- #233 every diagram shares a 1.5–74 s per-project read of `structure_edges`
  (zones 40 s on top of it). Matview + pg_cron is new infrastructure;
  refresh-on-index-barrier avoids it. This is the gap between demoable and usable.
- #234 nothing lets a developer ADOPT a seeded rule pack — no CLI, MCP tool,
  endpoint or screen writes `rule_pack_adoptions`. Two of fourteen seeded packs
  are therefore unreachable, and every user-authored or dōjō-shared pack will
  arrive the same way.

## Known broken

- **#231 an fqn's segment 3 is a SYMBOL for some adapters, not a module.** 13.7%
  of `sensei`'s module dependencies and 78% of the largest client project's
  point at a unit that owns no file. Counted as `coverage.unknownUnit`; the
  shipped Structure diagram has the same hole (31 of 213), uncounted.
- **dbd#41 blocks `dbd reconcile` on any database `dbd deploy` created.** It
  emits a false `ALTER TYPE` for a column whose extension type lives off the
  search_path — the configuration `design.yaml` asks for. Workaround: apply the
  one changed file with `psql -f`, or drop and redeploy `sensei_test`.
- **App e2e 114/21/17 skipped** — the known red gate (#187). Sampled causes are
  harness defects, not this slice.
- `--features senseid/embedded-llama-cpp` will not compile (#203). `EMBED=0`
  works on every make target and is how the live daemon was built.
- 30+ commits UNPUSHED on `develop`. Otherwise green: senseid 3270/0, clippy
  `-D warnings` + fmt, dbd doctor, SQL statements plan against a fresh deploy
  (and that planner now runs in CI).
- Two unattributed single-run failures, both passing on rerun and in isolation:
  `audit_repairs_nested_standalone` and `prune_empty_projects_…`. The latter uses
  fixed project names and runs a GLOBAL prune, which is the #183 class. The one
  race I could name — two tests fighting over the `cost.subscription` config key
  — is fixed with a mutex.
