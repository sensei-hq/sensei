# Checkpoint

**Slice:** Observatory diagrams — data → derivation → screens. Lane 1 (data) done.
Lane 2 (derivation) half done: cycles and layers ship, zones and the matrix do not.

## Next — autopilot queue (in order)

Order set 2026-10-03: core computation/indexing → layer semantics → sensei → dōjō.
Issues are the tracker; this is the position in the queue.

    [x] #215 registering a root enqueues a scan             91809876
    [x] #216 watcher batch needs no database                91809876
    [x] #224 git history scanner                            964a99b6
    [x] #222 derivation: SCC + layering                     b71dc0af
    [ ] #223 derivation: Zones + Dependency matrix   ← NEXT
    [ ] #227 persistence layer: fold in or document the exception
    [ ] #217 governance seed drift (ponytail adoption)
    [ ] #218 transcript ingestion opt-in
    [ ] #232 Diagrams · Layers + Cycles screens
    [ ] #219 World · #220 Neighbourhood · #221 Schema
    [ ] #225 dōjō team view — needs the sync-shape decision first

Per slice: TDD, mutation-probe each test, full suite + clippy + fmt +
`scripts/check-sql-against-schema.py`, then close the issue with evidence.

## Open questions

- #230 a DECLARED layering — a derived one provably cannot climb, so Layers has
  no violation to show until something stores an intended one. Who authors it,
  where it lives, and whether it belongs to the project or the repository.
- #233 every diagram shares a 1.5–74 s per-project read of `structure_edges`.
  Matview + pg_cron is new infrastructure; refresh-on-index-barrier avoids it.
- Should `ponytail` be adopted by default? Live has it, a fresh install does not
  (#217). `sensei.projects` has no `namespace_id`, so a rename orphans the slug.

## Known broken

- **#231 an fqn's segment 3 is a SYMBOL for some adapters, not a module.** 13.7%
  of `sensei`'s module dependencies and 78% of the largest client project's
  point at a unit that owns no file. Counted as `coverage.unknownUnit`; the
  shipped Structure diagram has the same hole (31 of 213), uncounted.
- **App e2e 114/21/17 skipped** — the known red gate (#187). Sampled causes are
  harness defects, not this slice.
- `--features senseid/embedded-llama-cpp` will not compile (#203). `EMBED=0`
  works on every make target and is how the live daemon was built.
- 30+ commits UNPUSHED on `develop`. Otherwise green: senseid 3267/0, clippy
  `-D warnings` + fmt, 1,539 SQL statements plan against a fresh deploy.
- `audit_repairs_nested_standalone` failed once on a full run, passed on rerun
  and in isolation. Unattributed; shared test DB (#183) is the likelier cause.
