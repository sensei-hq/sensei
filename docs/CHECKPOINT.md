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

## Next

    cargo test -p senseid --bins   # baseline before the next screen

Lane 3: the next Observatory diagram screen. Structure is the worked template.

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
