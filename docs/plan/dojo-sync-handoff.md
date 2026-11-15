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
  slice. ANSWERED C (user, 2026-11-03): push the diagram data, each payload tied to its commit and
  time, so the dōjō can show the latest and a progression across time. Built in WP6/WP7.
- **Q2. Diagrams per repository or per workspace?** Local diagrams are per workspace; dōjō graphs
  are per (repository, branch). **Recommend per repository first.**
- **Q3. D18 environments.** Use `repository_graph_branches.label ∈ {production, testing,
  development}`, or a separate table? **Recommend the label**, with a CHECK (dōjō scope). This
  affects WP3.
- **Q4. Gate 1 (G6).** ANSWERED by D46 (user, 2026-11-02): public repositories are free — an
  organisation's sync automatically (admin may turn one off); a personal dōjō is free, private
  included, its repositories the person's to choose, all by default; an organisation's private
  repositories need its admin/owner's subscription and are hidden until there is one. The tier is
  dōjō-only; local sensei works in full. The local `visibility` gate is retired. Built in WP8.
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

**WP6 · Diagram payload push (daemon + dōjō DB + Worker).** Q1 = C (user, 2026-11-03), with history:
every payload is tied to its commit and time, and none is overwritten. Depends on WP3 (without a
tracked `develop` there is nothing to verify on live data).
- Schema (`--scope dojo`): `dojo.graph_diagrams(repository_id, branch, view, head_sha, committed_at,
  payload jsonb, pushed_at)`, unique on `(repository_id, branch, view, head_sha)`, indexed on
  `(repository_id, branch, view, committed_at desc)`. One row per commit pushed — a later push adds a
  row, it never replaces one. `committed_at` is the commit's own time (git), not the push time, so a
  late push still lands at its place in the progression. Plus a SECURITY DEFINER
  `file_graph_diagrams` using the `file_graph_slices` contract (principal from the session, verdict
  from `all_my_repositories`, the whole batch refused on any failure); re-filing the same
  `(…, head_sha)` is a no-op, not an error. Add policy `database/policies/dojo/graph_diagrams.sql`.
- Reads (Worker): latest = the newest `committed_at` per (repository, branch, view); progression =
  the rows of one (repository, branch, view) in a window, oldest first, payloads optional (a
  `head_sha, committed_at` timeline first, a payload on demand) — it is the dōjō half of S6
  (diagram history and animation).
- Daemon: after a `Synced` checkout in `push_checkout`, compute the default-control payloads through
  the existing `diagrams.rs` functions and push them with the checkout's HEAD sha and commit time.
  Record the result under `repository_graph` (`partial` if one view fails). History accrues from the
  first push; no backfill of past commits in this slice.
- Tests first: the SQL test (`other_tenant_refused`, `unelected_refused`,
  `refiling_a_commit_changes_nothing`, `a_new_commit_adds_a_row_and_keeps_the_old`,
  `latest_is_the_newest_commit_not_the_newest_push`), `a_synced_checkout_pushes_its_diagram_payloads`,
  and `a_skipped_checkout_pushes_no_diagrams`.
- Accept: for each view, the latest row's `head_sha` matches the local HEAD; after a second commit
  is synced there are two rows per view, and the latest read answers the newer.
- Open (raise before building): retention — payloads grow per commit; propose keeping every commit
  for 90 days, then one per day. Needs the user's yes.

**WP7 · Diagrams screen (dōjō UI).** Depends on WP6. Read route plus a flat Diagrams list (D19) with
a branch picker over the tracked set, rendered with `@rokkit/graph`. Opens on the latest commit;
a timeline of the commits pushed lets the reader step back through the progression. Fix rendering defects in rokkit
upstream, never by overriding in the app.

**Parallelism.** These can run at the same time: WP1, WP3, WP4. WP2 follows WP1, WP5 follows WP4,
WP7 follows WP6.

**Conflict risk:**
- WP1 and WP6 both edit `tasks/dojo_sync.rs`, so run them one after the other.
- WP3, WP5 and WP7 may all touch `dojo/src/lib/nav.ts` and the `[section]` loaders.
- WP4 and WP6 both reconcile the same local Supabase, so do not run reconcile in parallel.

**WP8 · Sharing and plans follow D46 (dōjō DB + dōjō UI + daemon + app).** Independent of
WP1–WP7; touches `all_my_repositories.ddl`, which WP4 reads — sequence WP8 before WP4 if both are in
flight.
- `database/ddl/view/dojo/all_my_repositories.ddl`, two changes:
  1. ENTITLEMENT — a personal tenant's private repository is entitled, free: after the capture
     term, `when mr.origin = 'personal' then null`. This reverses the view's own note ("no
     `origin = 'personal'` → ALLOW … would have hosted every personal private repository free"):
     that is now the decision (D46). An organisation's private repository keeps every billing term
     (`unclaimed`, `not_subscribed`, `subscription_expired`). Rewrite the note to say so.
  2. ELECTION — an organisation's PUBLIC repository is the organisation's: authority
     `organization`, an absent decision ELECTED (`coalesce(org_elected, true)`),
     `configurable_by_me` admin-only. Today it is the member's (`coalesce(user_elected, true)`, any
     member may switch it off). A personal repository, public or private: unchanged (the person's,
     absent is elected).
  Converge with `dbd reconcile --scope dojo`, twice.
- dōjō Sharing (`ScrSharing`, `/you/sharing`) and every repository list: an ORGANISATION's private
  repository is hidden while the organisation has no subscription (`reason_code` `not_subscribed`
  or `unclaimed`); in its place one line — private repositories are supported on paid plans —
  linking to that organisation's Plan & billing. Hidden, not deleted: subscribing shows them with no
  re-registration. A personal dōjō's private repositories are listed and sync.
- Plan & billing (`ScrBilling`, the organisation's): the current plan reads **Free** with no billing
  account (today "No billing account yet"). Its copy — free where public or personal, paid where
  private and shared — already matches D46; keep it.
- Daemon/app: retire `sensei.repositories.visibility` as a gate. `PATCH /api/repositories/{key}`
  writes the dōjō election (`/v1/you/repositories/election`) where the person may change it and
  answers 409 where they may not; the app shows the sync toggle only where `configurable_by_me`.
  Nothing local is gated on the plan.
- Tests first: dōjō view `a_personal_private_repo_is_entitled_free`,
  `an_org_private_repo_without_a_subscription_is_refused_not_subscribed`,
  `an_org_public_repo_syncs_with_no_decision`,
  `an_org_public_repo_is_not_a_members_choice` (403 for a member, 200 for an admin),
  `a_personal_repo_syncs_by_default_and_its_opt_out_holds`;
  dōjō UI `sharing_hides_an_org_private_repo_until_subscribed_and_links_to_billing`,
  `sharing_lists_a_personal_private_repo`, `billing_reads_free_with_no_account`;
  daemon `patch_visibility_writes_the_election_where_the_person_may`,
  `patch_visibility_on_an_org_public_repo_is_refused`.
- Accept: on the live dōjō, `select origin, forge_visibility, elected, configurable_by_me,
  reason_code, count(*) from dojo.all_my_repositories group by 1,2,3,4,5` — every org public row
  `elected = true`, non-admin `configurable_by_me = false`; personal private rows entitled (no
  `not_subscribed`); org private rows on no subscription `not_subscribed`. The next sync pass's
  `allowed` grows by the personal private repositories that were refused.
- Do not: give a member a switch on an org repository; touch an organisation's billing terms;
  gate anything local on the plan; delete a hidden repository's data.

**WP9 · Registration that scales (dōjō DB + Worker + daemon).** Found 2026-11-03 against production:
the dōjō runs on Cloudflare Workers, which cap outbound calls PER REQUEST (50 on the free plan), and
registration costs 1 (caller) + 1 (`register_repository`) + 1 (`file_repository_workspace`) per
repository — 157 at once failed every pass with "Too many subrequests". The daemon now sends 15 per
request (fb92e3f39, ≤31 calls): a stopgap that guesses the server's budget from outside. And every
daemon re-registers EVERY repository on EVERY pass (~1–2 min): ~11 requests and up to ~340 database
calls a pass for one user, ~7–15k Worker requests a day — a handful of such users exhausts the free
plan's 100k/day. Concurrency is not the cap's problem (each request has its own budget); volume is.
- Part A — dōjō, one call per batch: a set-based `dojo.register_repositories(p_repos jsonb)` (returns
  the same mapped/unmapped rows, filing included) so a request costs ~2 calls whatever its size; the
  route calls it once, and answers its maximum batch (`max_batch`) so the daemon stops guessing.
  Tests first: SQL — `registering_many_is_one_call_with_the_same_answers_as_one_by_one`,
  `a_failed_filing_is_partial_for_that_repository_only`; route —
  `the_route_makes_one_database_call_for_a_batch`, `it_answers_its_maximum_batch`.
- Part B — daemon, register only what changed: remember each repository's last registration
  (key, remote, workspace → answer, at) and offer only new or changed ones, plus everything once a day
  (and after a sign-in) so a dōjō-side change is still learned. Verdict rows (WP1) still come from the
  pass's full picture — the cached answers stand in for the ones not re-asked. Use the dōjō's
  `max_batch` when it answers one, else 15. Tests first in `dojo_sync.rs` with the scripted plane:
  `an_unchanged_repository_is_not_re_offered`, `a_moved_workspace_is_re_offered`,
  `everything_is_re_offered_once_a_day`, `verdicts_cover_every_repository_even_when_few_were_offered`.
- Accept: against production, a pass after the first offers 0 repositories (log `offered=0`) while
  `sensei.sync_state` still holds a verdict for all mapped+unmapped; the Worker's subrequests per
  registration request stay ≤ 3 (Cloudflare dashboard → Workers → Metrics).
- Do not: raise the per-request cap by any means other than the batched call; drop a verdict for a
  repository that was not re-offered; register on every pass "to be safe".
- Depends on nothing; Part A ships with a dōjō release, Part B with a daemon release — B works
  against an older dōjō (falls back to 15), A against an older daemon (it simply sends batches).

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
