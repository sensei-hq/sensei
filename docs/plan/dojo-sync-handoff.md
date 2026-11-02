# sensei → dōjō sync: handoff plan (repositories, metrics, diagrams)

Written 2026-11-02 for an agent with repo access and no prior context. Read first: `CLAUDE.md`,
`docs/spec/dojo/daemon-sync.md` (§7, §8a–8c), `docs/spec/dojo/graph-sync.md`,
`docs/spec/dojo/access-matrix.md`, `docs/roadmap/09-dojo-expansion.md` (D5–D8, D18, Wave B).

## 1. What exists today (measured on the live DB and daemon log, 2026-11-02)

The **push already exists**. What is missing is (a) recording why a repository does not sync,
(b) graph sync for the branches people actually work on, and (c) any dōjō screen that reads the
pushed repository metrics or graph.

**Daemon cycle.** `crates/senseid/src/tasks/dojo_sync.rs` (module doc :1–56). `spawn` (:1000) runs
`tick` (:172) through the `dojo_sync` row in `sensei.schedules`. For each signed-in persona:
`persona_token` (:235, wraps `api/handlers/auth.rs:955 live_access_token`) → `plan_persona` (:367):
`POST /v1/you/repositories` (:436, every cloned repo with a `repo_key`) → `GET /v1/you/sync/plan` →
`push_allowed` (:821, `POST /v1/you/metrics`; watermark `sensei.repository_metrics.shared_at` through
`mark_metric_rows_shared`, `db/pg_store/sync.rs:484`) → `push_graphs` (:639) / `push_checkout`
(:689, `POST /v1/you/graph`, pure planning in `tasks/graph_push.rs`).

**Auth.** Bearer = the persona's Supabase access token (`dojo_client/user_plane.rs:233`). No device
credential. Target: `dojo_client/settings.rs:23 dojo_url()` (env `DOJO_URL`, default
`https://dojo.sensei-hq.com`).

**Retries.** No backoff: the next scheduled tick retries. Failures land in `sensei.sync_state`
(`database/ddl/table/sensei/sync_state.ddl`; states `pending|synced|partial|error|skipped`; one row
per `(entity, entity_key, direction)`; writers `mark_synced/…_error/…_partial/…_skipped`,
`sync.rs:129–232`).

**Status screen.** `GET /api/dojo/sync-state` (`api/handlers/dojo.rs:20`, `sync.rs:277`) →
`app/src/routes/(preferences)/settings/dojo/` (`dojo-status-state.svelte.ts` maps state → tone).

**Live state.** `sync_state`: `repository_metric` 17 synced; `dojo_sync_plan` 2 synced;
`repository_graph` 1 synced (`sensei@main`, 10-27), 5 skipped `not_a_tracked_branch` (every
`@develop` checkout); `forge_mirror` 2 error. Last cycle: `allowed=31 denied=25 mapped=56
unmapped=101 graph_batches=0`. Denials are `not_subscribed`; unmapped are `no_connection` and
`unknown_host`.

**Gaps, verified:**

- G1. Unmapped and denied repositories are only logged (`dojo_sync.rs:470`, `:494`). The `repository`
  sync_entity has no production writer (one test only, `pg_store/tests/dojo_tests.rs:918`). So 21
  mapped repos with metrics, and 40 unmapped ones, have no row anywhere and are absent from the
  status screen. A `workspace_error` on registration is also only logged.
- G2. `SyncPlan.denied`'s doc says "always empty in phase 1" (`user_plane.rs:167`), but 25 are
  denied today.
- G3. The dōjō tracks only the forge default branch (`dojo/src/lib/server/provisioning.ts:698
  trackDefaultBranch`). `dojo.set_graph_branch` exists and is tested
  (`database/tests/dojo/graph_branches_access.sql`), but no Worker route or UI calls it. D18's
  production/testing/development mapping is not built.
- G4. No dōjō code reads `dojo.graph_*`: only ingest uses it (`dojo/src/lib/server/graph-ingest.ts`).
  Local diagrams come from Rust (`crate::analysis::{codemap,layering,neighbourhood}`, served by
  `api/handlers/diagrams.rs` per workspace and cached by `api/diagram_cache.rs`).
- G5. The dōjō reads metrics only for "My metrics" (`dojo.my_metric_series`) and team views
  (`team_metric_totals`, `team_member_metrics`). No repository-scoped or workspace metric view
  exists. The workspace detail page shows the constitution only.
- G6. Gate 1 (`sensei.repositories.visibility`) does nothing: `shared_repositories` has no
  production caller (`sync.rs:632–660`).
- G7. Wave B (dōjō → sensei workspaces/teams pull) is not built. `workspace` and
  `workspace_repository` sync_entities have no writer.

## 2. Definitions

- **Repository sync.** Every cloned repo with a `repo_key` has exactly one `sync_state(repository,
  repo_key, push)` row saying synced / skipped(reason) / error / partial(workspace filing failed).
  The app shows the reason. Registration and filing themselves already work.
- **Metrics sync.** The push already works. The missing parts: the verdict is visible (G1), and
  repository-scoped metrics are readable in the dōjō by the people the access matrix allows (G5).
  Workspace metrics are a derivation over `repository_metrics` joined through
  `dojo.repositories_in_workspaces`, the same way `sensei.workspace_metrics` is a view locally.
  Nothing new is pushed.
- **Diagrams/data sync.** The graph (nodes, edges, files, `graph_schema_*` for the ER/"data"
  diagram) reaches the dōjō for the branches a team tracks (G3), and the dōjō renders diagrams from
  it (G4). How it renders is Q1.

## 3. Open questions for the user (each has a recommended answer)

- **Q1. How does the dōjō get diagram payloads?** The derivation is Rust in the daemon. (A) Port it
  to SQL/TS on the dōjō: this duplicates `crate::analysis`, which the DRY rule forbids without
  approval. (B) Compile the analysis to WASM for the Worker: one implementation, but heavy reads on
  every request. (C) The daemon also pushes derived payloads per (repository, tracked branch, head
  sha, view) at default controls; the raw graph stays as the source. **Recommend C** as the first
  slice. It partly revisits the 2026-10-14 "raw graph only" decision, so it needs your yes. WP6/WP7
  wait on this.
- **Q2. Diagrams per repository or per workspace?** Local diagrams are per workspace; dōjō graphs
  are per (repository, branch). **Recommend per repository first.**
- **Q3. D18 environments.** Use `repository_graph_branches.label ∈ {production, testing,
  development}`, or a separate table? **Recommend the label**, with a CHECK (dōjō scope). This
  affects WP3.
- **Q4. Gate 1 (G6).** ANSWERED by D46 (user, 2026-11-02), for PUBLIC repositories: an
  organisation's sync automatically and no member controls them; a personal dōjō's are the
  person's to choose, all by default. Private repositories stay gated on billing, unchanged. The
  local `visibility` gate is retired; the dōjō election is the one authority. Built in WP8.
- **Q5. Wave B (G7).** Is it part of "repository sync"? **Recommend no**: it is a separate epic
  (roadmap Wave B).
- **Q6. The 101 unmapped repos.** They are configuration, not code: connect the forge orgs
  (the client organisations, …) in dōjō Integrations. Confirm that is intended.

## 4. Work packages

Rules for every package: write the failing test first and watch it go red, then implement, then
commit the test with the code when green. Zero errors. Production SQL only in
`crates/senseid/src/db/pg_store/`. No fabricated defaults on failure paths. Do not use
`--no-verify`.

**WP1 · Record each repository's verdict (daemon).** No schema change: the `repository` enum value
exists.
- Files: `tasks/dojo_sync.rs` (`plan_persona` :436–553), `db/pg_store/sync.rs` (a new batched
  `mark_repository_verdicts(&[…])`, one upsert statement, because 157 rows a minute must not be 157
  round trips), `dojo_client/user_plane.rs:167` (fix the stale doc).
- Tests first, in `dojo_sync.rs` tests with the scripted plane at :1277:
  `an_unmapped_repository_is_recorded_skipped_with_its_reason` (`last_error = "unmapped:no_connection"`),
  `a_denied_repository_is_recorded_skipped_with_the_dojo_reason`,
  `an_allowed_repository_is_recorded_synced`,
  `a_failed_workspace_filing_is_partial_and_names_the_error`,
  `a_repository_that_later_maps_flips_from_skipped_to_synced`,
  `a_failed_plan_fetch_leaves_repository_rows_untouched` (do not overwrite on failure).
- Accept: `select state, split_part(last_error,':',1), count(*) from sensei.sync_state where
  entity='repository' group by 1,2` adds up to the cycle's `mapped+unmapped` (157 today).
- Do not: write an `error` row for a deliberate refusal. Refusals are `skipped`.

**WP2 · Status screen shows repository verdicts (app).** Depends on WP1.
- Files: `app/src/routes/(preferences)/settings/dojo/*`, `app/src/lib/types.ts`.
- Group rows by entity, show counts per state, show the reason. Map the remedy from
  `sensei.reason_codes` (domain `repository_sharing`) by joining in `sync_state_rows`. Do not add a
  second copy of the remedy text in TS.
- Tests first: `dojo-status-state.spec.svelte.ts`: `groups_rows_by_entity_with_state_counts`,
  `a_skipped_repository_reads_muted_with_its_remedy`, `an_unknown_reason_passes_through_verbatim`.
- Accept: the screen in the installed app (`make app-e2e-bundle`) shows 157 repository rows grouped
  under one heading. Quit the installed Sensei before inspecting.

**WP3 · Tracked graph branches (dōjō Worker + UI).** Independent of WP1/WP2. Schema change only if
Q3 adds the CHECK (`--scope dojo`).
- Files: new `dojo/src/routes/v1/you/repositories/graph-branches/+server.ts` (PUT/DELETE calling
  `dojo.set_graph_branch`), `dojo/src/lib/server/repositories.ts`, and the Sharing section
  (`dojo/src/routes/(app)/you/[section]/+page.ts:83` and its `.svelte`).
- Tests first: a route spec covering `adds_a_tracked_branch_for_a_configurable_repository`,
  `refuses_a_member_without_authority_with_403`, `an_empty_branch_is_400`. Extend the SQL test
  `graph_branches_access.sql` only for the Q3 CHECK.
- Accept: after adding `develop` to `github.com/sensei-hq/sensei`, the next tick shows
  `sync_state(repository_graph,'github.com/sensei-hq/sensei@develop')='synced'`. Then run the
  graph-sync §7 conservation check: per-table dōjō row counts equal the local counts for that
  repository, and no path starts with `/`.
- Do not: guess `main`. Untracking deletes that branch's graph, so the UI must ask for an in-page
  confirmation first.

**WP4 · Repository-metric read (dōjō DB + Worker).** Independent.
- Schema (`--scope dojo`): add `dojo.repository_metric_series(p_repository_id uuid, p_since date)`
  as SECURITY INVOKER over `repository_metrics`, gated by `can_read_repository_metric`. Add a
  workspace rollup over `repositories_in_workspaces` that is aggregate-only for viewers (D6/D8). Add
  grant and comment lines (`make check-ddl-grants check-ddl-comments`).
- Tests first: `database/tests/dojo/repository_metric_series.sql`:
  `role_holder_reads_repo_rows`, `contributor_does_not_read_a_peers_user_rows`,
  `maintainer_reads_peers_user_rows`, `viewer_gets_aggregates_only`,
  `non_member_reads_nothing`. Plus a Worker route spec.
- Route: `GET /v1/t/[origin]/[org]/workspaces/[id]/metrics`. A read failure is a 5xx, never `[]`.
- Accept: on the local Supabase with synced data, the route returns `sensei-hq/dbd`'s repo rows
  for a member and 403 for an outsider.

**WP5 · Workspace metrics screen (dōjō UI).** Depends on WP4.
- Files: `dojo/src/routes/(app)/org/[slug]/workspaces/[id]/*`, plus new `lib/workspace-metrics-{data,view}.ts`.
  Build it presentation | state | load, using rokkit components first.
- Tests first: `-view.spec.ts` (shaping and per-repository split) and a load spec where a failed read
  shows the error state.
- Accept: the workspace page shows per-repository charts for the sensei-hq workspace.

**WP6 · Diagram payload push (daemon + dōjō DB + Worker).** Blocked on Q1 = C. Depends on WP3
(without a tracked `develop` there is nothing to verify on live data).
- Schema (`--scope dojo`): `dojo.graph_diagrams(repository_id, branch, view, head_sha, payload
  jsonb, pushed_at)` plus a SECURITY DEFINER `file_graph_diagrams` using the `file_graph_slices`
  contract (principal from the session, verdict from `all_my_repositories`, the whole batch refused
  on any failure, an older sha refused). Add policy `database/policies/dojo/graph_diagrams.sql`.
- Daemon: after a `Synced` checkout in `push_checkout`, compute the default-control payloads through
  the existing `diagrams.rs` functions and push them. Record the result under
  `repository_graph` (`partial` if one view fails).
- Tests first: the SQL test (`other_tenant_refused`, `unelected_refused`,
  `older_head_refused`), `a_synced_checkout_pushes_its_diagram_payloads`, and
  `a_skipped_checkout_pushes_no_diagrams`.
- Accept: `select view, head_sha from dojo.graph_diagrams` matches the local HEAD for each view.

**WP7 · Diagrams screen (dōjō UI).** Depends on WP6. Read route plus a flat Diagrams list (D19) with
a branch picker over the tracked set, rendered with `@rokkit/graph`. Fix rendering defects in rokkit
upstream, never by overriding in the app.

**Parallelism.** These can run at the same time: WP1, WP3, WP4. WP2 follows WP1, WP5 follows WP4,
WP7 follows WP6.

**Conflict risk:**
- WP1 and WP6 both edit `tasks/dojo_sync.rs`, so run them one after the other.
- WP3, WP5 and WP7 may all touch `dojo/src/lib/nav.ts` and the `[section]` loaders.
- WP4 and WP6 both reconcile the same local Supabase, so do not run reconcile in parallel.

**WP8 · Public-repository sharing follows D46 (dōjō DB + daemon + app).** Independent of WP1–WP7;
touches `all_my_repositories.ddl`, which WP4 reads — sequence WP8 before WP4 if both are in flight.
- `database/ddl/view/dojo/all_my_repositories.ddl`: an organisation's PUBLIC repository is the
  organisation's, not the member's — authority `organization`, an absent decision ELECTED
  (`coalesce(org_elected, true)`), `configurable_by_me` admin-only (today it is the member's,
  `coalesce(user_elected, true)`, any member may switch it off). A personal public repository
  keeps today's rule. PRIVATE repositories are untouched: billing (entitlement) and the
  organisation's policy (#301) decide them. Converge the dōjō schema per its flow
  (`dbd reconcile --scope dojo`), twice.
- Daemon/app: retire `sensei.repositories.visibility` as a gate. `PATCH /api/repositories/{key}`
  writes the dōjō election (`/v1/you/repositories/election`) where the person may change it and
  answers 409 where they may not; the app shows the sync toggle only where `configurable_by_me`.
- Tests first: dōjō view specs `an_org_public_repo_syncs_with_no_decision`,
  `an_org_public_repo_is_not_a_members_choice` (403 from `set_repository_election` for a member),
  `a_personal_public_repo_syncs_by_default_and_its_opt_out_holds`,
  `an_org_private_repo_is_still_decided_by_billing_and_policy` (unchanged verdicts);
  daemon `patch_visibility_writes_the_election_where_the_person_may`,
  `patch_visibility_on_an_org_public_repo_is_refused`.
- Accept: on the live dōjō, `select origin, forge_visibility, elected, configurable_by_me, count(*)
  from dojo.all_my_repositories group by 1,2,3,4`: every org public row `elected = true` and, for a
  non-admin, `configurable_by_me = false`; private rows' verdicts unchanged from before.
- Do not: give a member a switch on an org repository; touch entitlement or the private rules.

## 5. Context you need

- Build and test:
  - `make test-fast` does **not** cover senseid. Gate a commit on `cargo test -p senseid` (full).
  - The DB tests clone `sensei_test`. Run `make test-template` after a `default`-scope DDL change.
  - App: `make app-check test-app-unit`. dōjō: `cd dojo && bun run check && bun run test`.
  - SQL: `make test-db` (local Supabase, `database/tests/run.sh dojo`).
  - `make check-sql-in-layer` enforces where SQL may live.
- Local sensei DB (pre-release):
  - `cd database && dbd reconcile -d postgresql://localhost:5432/sensei --scope default --dry-run`,
    then the same without `--dry-run`, then `dbd import … --scope default`.
  - Never hand-applied psql.
- Local dōjō:
  - `make dojo-local` runs Supabase plus `bun run dev` on `127.0.0.1:5173`.
  - Schema: `cd database && DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:54322/postgres
    ../scripts/deploy-dojo-db.sh`. This is what `release.yml` runs. The dōjō is pre-v1, so
    reconcile is `--allow-destructive`.
  - **Deploy twice**: once on an empty database, and again on one that already has the objects.
  - Point the daemon at it with `DOJO_URL=http://127.0.0.1:5173`. The runbook is `dojo/CLAUDE.md`
    §"Local dōjō for the daemon".
- Daemon:
  - `make install-debug` replaces the binary, which revokes Keychain approval.
  - Watch `~/.sensei/logs/senseid.<date>.log` for `dojo_sync: cycle complete`.
- Gotchas:
  - A test that spawns `git` must clear every `GIT_*` env var. Under the pre-commit hook a child git
    otherwise writes to the real repo.
  - Supabase `service_role` bypasses RLS, so every Worker read must use an invoker function or an
    explicit principal filter.
  - A `null === null` fake (`fake-dojo-db.ts`) is stricter than Postgres. Prove uniqueness in SQL
    tests.
