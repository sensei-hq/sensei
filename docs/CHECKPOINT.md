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
    [ ] #224 git history scanner  ← BUILDING
        [x] 1 DDL: commits · commit_files · commit_scans   (applied, 3 DBs)
        [x] 2 git helper: exit-code granularity, 19 tests, mutation-probed
        [x] 3 numstat parser: -z bytes, 24 tests; braces 2,087→0, cquote 30→0
        [x] 4 ScanGitHistory TaskKind + handler + enqueue (3 tests, probed)
        [ ] 5 migrate churn.rs onto the shared parser  ← LAST ITEM
            DRY is VIOLATED until this lands: two numstat parsers exist.
            Needs the parser to also carry the COMMITTER date — churn
            buckets on %cd, history on %aI (survives rebase). Migrating
            without that silently moves churn values between days.
        [x] 6 retention prune: 400d default, floor enforced, 4 tests
        [ ] 7 gates + close #224   (6fdfd84d landed items 1,2,3,4,6)
        v1 pruned commits by `rev-list HEAD` reachability → 28 problems. Killed.
        v2 = INSERT-ONLY and validated. Why the prune was never needed: 4,298 of
        6,654 paths ever touched in sensei (65%) no longer exist, so they have
        no `files` row and cannot reach a diagram. EXISTENCE is the filter, not
        reachability.
        Walk the TIP SET (`--branches --remotes --tags`), not HEAD and not
        `--all`: 3 of 4 co-keyed checkout pairs then produce byte-identical
        commit sets (symmetric difference 0; the 4th is 1.7%), so two checkouts
        agree instead of fighting. `--all` sweeps `refs/stash` + `refs/original`.
        Tables: commits(repository_id, sha, authored_at, author_email) ·
        commit_files(repository_id, sha, path, lines_changed) ·
        commit_scans(folder_id PK — cursor is per CHECKOUT, facts per repository).
        authored_at not committed_at: survives rebase/cherry-pick so one logical
        change buckets identically through two checkouts. Diverges from churn.rs
        (committer date) — named, not silent. No file_id (65% would be NULL).
        Decisions: cap 50 · window 90 configurable · ownership configurable
        (commits/lines/recency, default commits) · retention 400d with an
        enforced `retention_days >= max(window_days)` floor.
        Folds in two live churn.rs bugs: 2,087/24,496 records (8.5%) are rename
        pseudo-paths stored verbatim, 30 more are C-quoted octal; and
        `unwrap_or(0)` reports a binary file's UNKNOWN churn as zero. `-z` + one
        shared parser fixes both (DRY forbids a second numstat parser).
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
