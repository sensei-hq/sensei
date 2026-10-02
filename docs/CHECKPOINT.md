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
- **#205, #206 closed** earlier in the slice (Structure endpoint + screen).

## Next

    cargo test -p senseid --bins   # baseline before the next screen

Lane 3: the next Observatory diagram screen, from `docs/backlog.md`. Structure
is the worked template (spec + handler + `*-state.svelte.ts` + e2e).

## Open questions

- `sensei.projects` has no `namespace_id`, so a project's namespace resolves by
  NAME. Correct today; a rename would orphan the dōjō slug. Worth filing.
- 231 uncommented columns filed; #213/#214 and #201–#204 open.

## Known broken

- `make install-debug` / `crates-debug`: vendored `llama-cpp-sys-2`
  (cpp-httplib) will not compile against the system OpenSSL headers.
  Pre-existing, unrelated. `cargo build -p senseid -p sensei-cli -p sensei-mcp
  --bins` works and is what the running daemon was deployed from.
- 30+ commits UNPUSHED on `develop` (nothing to the remote since the scrub).
- Otherwise green: senseid 3197/0, workspace 361/0, clippy `-D warnings` and
  `fmt --check` clean, `dbd reconcile` reports no drift on either database.
