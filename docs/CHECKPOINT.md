# Checkpoint

**Slice:** Observatory diagrams — data → derivation → screens. Lane 2 done: a
project resolves one way, and so does governance.

## Done

- **#211 CLOSED — `folders.project_id` dropped** (17721622). Membership is the
  repository's. Dropped, not backfilled: 601 folders disagreed with the
  junction in one snapshot, zero an hour earlier — a second WRITER. Three LIVE
  defects it exposed, all fixed: governance leaked across projects (a bare
  `project_id` in a subquery bound OUTWARD, making the predicate `x = x` — one
  folder got 9 rules from 8 projects, answer 2); the nested-standalone healer
  aborted on the first project-less repo; `upsert_folder` put a project uuid in
  `workspace_root_id`.
- **Governance regrained to project + repository** (be1b4508).
  `repository_namespaces` + `sensei.namespaces_for_folder()`. All 447 old
  `folder_namespaces` rows sat on repo-ROOT folders — the grain was never used,
  it only let two checkouts disagree.
- **Clean-slate verification** (2321920d). Dropped and redeployed every DB from
  the dbd tree, then ran the real flows. Found: indexing worked and MEMBERSHIP
  DID NOT — 27,860 nodes, junction empty, every project `orphaned` with an
  empty Structure diagram. `ProcessGitFolder` ran before the reconcile assigned
  repositories. The handler now creates the repository itself. After: junction
  3, folder_projects 457, diagram 1,760 nodes. `indexer::pipeline::scan_root`
  (v2, #130) has NO production caller — that is why no test caught it.
- **Dōjō plane clean** (5a0aca08). Local Supabase + `dbd reset`/`deploy --scope
  dojo`: 115 entities, 5 RLS files, object diff ZERO. Dōjō app runs on the
  fresh DB, 1,535 unit tests green, all three relay endpoints the daemon calls
  exist and fail closed 401. Schema diff live vs fresh is now zero everywhere.

## Next — autopilot queue (in order)

Order set 2026-10-03: core computation/indexing → layer semantics → sensei → dōjō.
Issues are the tracker; this is the position in the queue.

    [x] #215 registering a root enqueues a scan        91809876
    [x] #216 watcher batch needs no database           91809876
    [x] #224 git history scanner — CLOSED
        Insert-only commits/commit_files/commit_scans, a -z byte parser, a git
        helper with exit-code granularity, ScanGitHistory, and 400d retention
        with an enforced `retention >= max(window)` floor.
        churn.rs migrated onto the shared parser, so ONE numstat reader exists.
        LIVE on this machine: 70,679 commits · 631,465 touches · 80 checkouts ·
        77 repositories (3 repos with 2 checkouts each — separate cursors, one
        fact set, which is the whole design). 0 brace pseudo-paths and 0
        C-quoted paths where the old reader produced 8.6%. 27,752 binary files
        NULL (unknown) vs 19,986 genuine zero-line diffs — the old
        `unwrap_or(0)` conflated all 47,738. Co-change: 7,287 pairs in 649 ms
        at query time with the 50-file cap, so nothing is materialised.
    [ ] #222 derivation: SCC + layering (Cycles, Layers)
    [ ] #223 derivation: Zones + Dependency matrix
    [ ] #227 persistence layer: fold in or document the exception
    [ ] #217 governance seed drift (ponytail adoption)
    [ ] #218 transcript ingestion opt-in
    [ ] #219 Diagrams · World
    [ ] #220 Diagrams · Neighbourhood
    [ ] #221 Diagrams · Schema
    [ ] #225 dōjō team view — needs the sync-shape decision first

Per slice: TDD, mutation-probe each test, full suite + clippy + fmt +
`scripts/check-sql-against-schema.py`, then close the issue with evidence.

## Open questions

- Should `ponytail` be adopted by default? Live has it, a fresh install does
  not (#217). `sensei.projects` has no `namespace_id`, so a project's namespace
  resolves by NAME — a rename would orphan the dōjō slug.
- Not verified: an authenticated daemon→dōjō round-trip (needs a membership +
  API key).

## Known broken

- **App e2e 114/21/17 skipped** — the known red gate (#187, was 23). Sampled
  causes are harness defects and an empty-DB precondition, not this slice; no
  before/after baseline was taken, so that is a judgement, not a measurement.
- `--features senseid/embedded-llama-cpp` will not compile (vendored
  cpp-httplib vs system OpenSSL). `EMBED=0` works on every make target and is
  how the current daemon and e2e app were built — so the installed daemon has
  NO in-process llama until this is fixed.
- #215 registering a root never enqueues a scan; #216 the watcher drops its
  first batch before PgStore is attached.
- 30+ commits UNPUSHED on `develop`. Otherwise green: senseid 3198/0,
  clippy `-D warnings` + fmt clean, 1,529 SQL statements plan against a fresh
  deploy with 0 failures.
