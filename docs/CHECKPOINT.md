# Checkpoint

**Slice:** Observatory diagrams — data (indexer) → derivation (views) → screens.
Lane 2 is done: a project resolves ONE way, and so does governance.

## Done

- **#211 CLOSED — `folders.project_id` dropped** (17721622). Membership is the
  repository's: `folders.repository_id` → `project_repositories` → `projects`.
  Dropped, not backfilled, because 601 folders disagreed with the junction in
  one snapshot and zero an hour earlier — a second WRITER, not stale data.
  Three LIVE defects it exposed, all fixed: governance leaked across projects
  (a bare `project_id` in a subquery bound OUTWARD to `m.project_id`, so the
  predicate read `x = x` — one folder got 9 rules from 8 projects, answer 2);
  the nested-standalone healer aborted on the first project-less repo (nullable
  `sole_project_of` decoded as a bare `Uuid`); and `upsert_folder` kept a bind
  after dropping its column, putting a project uuid in `workspace_root_id`.
- **Governance regrained to project + repository** (be1b4508).
  `folder_namespaces` dropped; `repository_namespaces` holds the 145 repository
  facts; `sensei.namespaces_for_folder()` is the single lift from a cwd. All
  447 old rows sat on repo-ROOT folders — the grain was never used, it only let
  two checkouts disagree. The 294 project-scope bindings went rather than
  moved: they restated project membership and 8 already contradicted it.
  Verified live: `/api/knowledge/rules` for this repo returns 29 general +
  exactly 1 project rule, its own.
- **Clean-slate verification** (2321920d). Dropped and redeployed `sensei_test`
  and `sensei_e2e` from the dbd tree alone, then ran the real flows against a
  virgin DB with `~/Developer/sensei-hq` registered. Found the big one:
  indexing worked and MEMBERSHIP DID NOT — 3 repos, 27,860 nodes,
  `project_repositories` empty, every project `orphaned` with `repos_count: 0`
  and an empty Structure diagram. `ProcessGitFolder` ran before `scan_root`'s
  reconcile assigned repositories, so `set_folder_project` matched no row.
  The handler now creates the repository itself. Verified end-to-end after the
  fix: junction=3, folder_projects=457, diagram 1,760 nodes / 1,282 edges,
  21 rules resolving.
  - `indexer::pipeline::scan_root` (v2, #130) has NO production caller — the
    #211 junction write lived there, which is why no test caught this.
  - Schema diff live vs fresh: tables and views identical; only two retired
    seed functions linger in live (#217).
- **Dōjō plane verified** (2026-10-02). Local Supabase started, `dbd reset
  --force --scope dojo` + `dbd deploy --scope dojo` against it: 115 entities,
  5 RLS policy files applied, and the before/after object diff is ZERO — 78
  objects both sides, no manual bypass on that plane either. Dōjō app runs
  against the fresh DB (routes 200, auth gate correct, `/version` 0.11.0), its
  1,535 unit tests green. The three relay endpoints the daemon calls
  (`relay/{session,segments,inbox}`) all exist and fail closed with 401;
  `artifacts` targets dojo-mind, a separate service, by design.
  NOT verified: an authenticated daemon→dōjō round-trip (needs a real
  membership + API key).
- **App e2e: 114 passed / 21 failed / 17 skipped** (152, 22.3 min) against a
  daemon-provisioned `sensei_e2e`. Consistent with the known red gate (#187,
  previously 23). Sampled failures are harness defects (an invalid
  `[data-pw-text={}]` selector, missing testids) and an empty-DB precondition
  — none related to this slice. No before/after baseline was taken this
  session, so "no regression" rests on those causes, not a measurement.
- **#205, #206 closed** earlier in the slice (Structure endpoint + screen).

## Next

    cargo test -p senseid --bins   # baseline before the next screen

Lane 3: the next Observatory diagram screen, from `docs/backlog.md`. Structure
is the worked template (spec + handler + `*-state.svelte.ts` + e2e).

## Open questions

- `sensei.projects` has no `namespace_id`, so a project's namespace resolves by
  NAME. Correct today; a rename would orphan the dōjō slug. Worth filing.
- 231 uncommented columns filed; #213/#214, #201–#204 open.
- NEW from the clean-slate run: #215 registering a root never enqueues a
  scan; #216 the watcher drops its first batch before PgStore is attached;
  #217 governance seed drift (ponytail adoption missing from the import).

## Known broken

- `llama-cpp-sys-2` (vendored cpp-httplib) will not compile against the system
  OpenSSL headers, so any build with `--features senseid/embedded-llama-cpp`
  fails. Pre-existing. WORKAROUND: `EMBED=0` on any make target skips the
  feature — `make install-debug EMBED=0` and `make app-e2e-build EMBED=0` both
  succeed. CONSEQUENCE: the installed daemon has NO in-process llama; it
  cannot be rebuilt with it until this is fixed.
- 30+ commits UNPUSHED on `develop` (nothing to the remote since the scrub).
- Otherwise green: senseid 3198/0, workspace 361/0, clippy `-D warnings` and
  `fmt --check` clean. `scripts/check-sql-against-schema.py` plans all 1,529
  SQL statements against a fresh deploy with 0 schema failures.
