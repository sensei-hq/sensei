# Review — WP4 · Repository-metric read (Muse, uncommitted)

Reviewed 2026-11-03 against `docs/plan/dojo-sync-handoff.md` §WP4. The work is **uncommitted** in the
develop checkout. Muse's process (pid 24357) is no longer running.

## Status — round 2 (2026-11-03, against Muse's staged index)

| Finding | Status |
|---|---|
| F1 unordered paging | **fixed**: five-key `.order(…)` chain |
| F2 viewer check copied | **fixed**: calls `is_workspace_viewer` |
| F3 pooled vs detail share `scope='user'` | **fixed**: pooled rows are `scope 'pooled'` |
| F4 pool of one | **REOPENED: the user answered YES (2026-11-03). Withhold a viewer pool with `pooled_rows < 2`.** See below |
| F5 tenant before caller | **fixed**: caller first |
| F6 malformed id → 500 | **fixed**: 400. *New nit:* `UUID_RE` duplicates `forge-azure.ts:152`. Extract one `isUuid` in `$lib/server` and use it in both places |
| F7 repeated joins | **fixed**: `with ws as (…)`, and both pooling comments now name each other |
| F8 lead/admin cases | open: confirm or add |

**BLOCKER for the commit: unrelated files in the index.** `packages/codemap/focus.ts` and
`focus.spec.ts` are staged. They are the diagrams session's work (the "focus filters every lens" slice),
not WP4. Unstage them with `git reset -q HEAD -- packages/codemap/focus.ts packages/codemap/focus.spec.ts`.
That only unstages them; the working tree is left as it is. Do not use `git restore` here (house git rule). Then confirm `git diff --cached --stat` lists
only WP4 paths and `docs/decisions.md`.

**F4: the user's answer overrides the decisions entry Muse added.** The staged
`docs/decisions.md` entry "WP4 F4 needed no new decision: D10 shows viewer pools of one" was correct
to cite D10 (D10 said revisit "only if comparison charts across people or teams are added"). The
user has now revisited it and said yes. Teams, sprints and gates add the cross-team comparisons D10
named as its trigger. Rewrite that entry as:

> **2026-11-03 — Workspace-viewer pools of fewer than 2 contributors are withheld (supersedes D10 for
> viewer pools).** The user, answering WP4 review F4. A viewer (D8) is often a client or a manager.
> A pool of one person is that person's own number, and once teams and sprints are compared, the
> comparison D10 deferred to has arrived. Detail readers (role holders, maintainers, admins, leads)
> are unaffected. `team_metric_totals`' k ≥ 3-others floor is a different shape and stays as it is.
> Alternatives: keep D10 (rejected); N = 3 (rejected: a viewer is not a member, so there is no
> subtraction attack and 2 is enough).

Implementation, red first: in `repository_metric_series.sql`, change the yesterday's-pool case so that
`client` gets **no** pooled row for `current_date - 1`, while today's pool (3 rows) still arrives.
Watch the test fail, then add `having count(*) >= 2` to the pooled branch. Update the function
comment ("pools of fewer than 2 are withheld: D10 superseded 2026-11-03") and fix the wire doc on
`WorkspaceMetricRow`. A withheld pool is a missing row, so WP5 must draw that day as a gap, not as
a zero.

## Scope reviewed

| File | State |
|---|---|
| `database/ddl/function/dojo/repository_metric_series.ddl` | new: SECURITY INVOKER, per repository |
| `database/ddl/function/dojo/workspace_metric_series.ddl` | new: SECURITY DEFINER, workspace rollup |
| `database/tests/dojo/repository_metric_series.sql` | new: the 5 named cases from the plan |
| `database/tests/dojo/definer_functions_gate_the_caller.sql` | modified: registers the new definer |
| `dojo/src/lib/server/workspaces-data.ts` (+ `.spec.ts`) | `getWorkspaceMetrics` + 4 specs |
| `dojo/src/lib/server/dojo-auth.ts` | `resolveTenantId` exported |
| `dojo/src/routes/v1/t/[origin]/[org]/workspaces/[id]/metrics/` | `+server.ts` + route spec |

**Not WP4, but in the same working tree:** `packages/ui/{Notice.svelte,notice.js,glyph-mode.js,index.js}`,
`packages/glyphs/index.js`, `app/src/lib/{glyphs.ts,notice.spec.ts,components/Notice.*}`,
`app/e2e/tests-inspect/schema-inspect.spec.ts`, `dojo/src/routes/+layout.svelte` (setGlyphMode),
`docs/CHECKPOINT.md`. Commit WP4 by itself. Stage these paths explicitly and check `git diff --cached --stat`.

## What was verified

- `bun run test` on both specs: **28/28 pass**.
- `repository_metric_series.sql` and `definer_functions_gate_the_caller.sql` on local Supabase
  (`:54322`, `psql -v ON_ERROR_STOP=1`): **both exit 0**. Both functions are deployed there.
- `make check-ddl-grants check-ddl-comments`: **clean**.
- **Not verified: the acceptance step.** No one has called the route against live data. Local Supabase has
  1,021 `github.com/sensei-hq/dbd` metric rows, so the step can be run (see "Before commit").

## What is good

- The per-repository read is SECURITY INVOKER over the table's RLS. The rule has one definition, so it
  cannot drift.
- The detail branch calls `can_read_repository_metric` and does not copy its logic.
- Each failure has its own status: 403 from the gate, 404 for the tenant scope, 500 when the read
  fails. A failed read is never `[]`, and the specs cover each case.
- It reuses `windowDays`, `windowSince`, `num`, `readAllPages`, `callFn` and `requireInTenant` rather
  than copying them.
- The pooling rule (Σ value×den / Σ den, null unless every denominator is > 0) matches
  `team_member_metrics`, and the test pins it (15/6, plus a pool with no denominator).
- The new definer is registered in `definer_functions_gate_the_caller.sql` for all three caller kinds.

## Findings (most severe first)

### F1 · HIGH · correctness: paging over an unordered set can drop or repeat rows
`workspaces-data.ts:295–300` + `workspace_metric_series.ddl` (no ORDER BY).
`readAllPages` sends one request per 1,000-row page (`repositories.ts:221`). Each page re-runs the
`UNION ALL … GROUP BY` with a fresh OFFSET, and without an ORDER BY Postgres does not keep the row order
the same between runs. Once a response passes 1,000 rows, a row can show up on two pages or on none,
and no error is raised. Local numbers today, per workspace, 90-day window: gateway 802, sensei 769,
dbd 624. Those are single-repository workspaces, already close to the limit. A dōjō admin reads every
detail row, so a workspace that holds two of these repositories goes past 1,000. The doc comment on `WorkspaceMetricsWire` also says "newest window first by day", which
nothing enforces.
**Fix (red first):** add a deterministic total order to the PostgREST chain:
`.order('repository_id').order('metric_key').order('scope').order('computed_on').order('principal_id', { nullsFirst: true })`.
This is unique per row: within one branch the table's natural key is unique, and a pooled row has a
null principal where a detail row has a real one. Test it with a spec that asserts the chain is
ordered. Also check whether the other unordered `readAllPages` callers have the same bug
(`my_repositories`, `my_graph_branches`, `graph_file_hashes`). Fix those in a separate commit.

### F2 · HIGH · house rule (DRY): the viewer check is written out twice
`workspace_metric_series.ddl:84–87` and `:115–118` inline
`exists (select 1 from dojo.workspace_viewers wv where … = dojo.current_principal_id())`.
That is exactly the body of `dojo.is_workspace_viewer(p_workspace)`, which `can_read_workspace`
already uses. If the viewer rule changes (expiry, revocation, a `disabled_at`), these copies will
drift from it. **Fix:** `and dojo.is_workspace_viewer(p_workspace_id)` in both branches.

### F3 · MEDIUM · data correctness (a trap for WP5): pooled rows and detail rows share `scope='user'`
A caller who is both a maintainer and a viewer (dave in the test) gets per-person `scope='user'` rows
**and** pooled `scope='user'` rows (principal null) that sum the same people. The test asserts this
on purpose (`:216–220`). A screen that sums `scope = 'user'` will count them twice. Also, in a pooled
row `value` is Σ of the rates (7 for rates 1, 2, 4), which means nothing for a rate metric. The real
rate is only in `props.numerator/denominator`.
**Fix (recommended):** give pooled rows their own scope (`'pooled'`), or add a `pooled boolean` column.
Then the wire type tells the two apart and WP5 cannot mix them up. At the least, WP5's
`-view.spec.ts` must have a case where a maintainer who is also a viewer is not double-counted.

### F4 · MEDIUM · privacy (needs the user's decision): a pool of one is one person's number
The viewer-pooled branch has no minimum group size. When only one person has a row that day, the
"aggregate" is that person's own value. The test fixture shows this: yesterday's pool is
`pooled_rows = 1`, which is alice's 5, shown to `client`, a pure viewer. The function comment
promises "never another person's row". D8 says a viewer reads "aggregated or split by repository",
and it does not say whether a pool of one counts.
**Question for the user:** should a pool smaller than N (recommend **N = 2**) be withheld from viewers?
Note that a solo repository's repo-scoped rows already show that one person's work, so this may be
acceptable. Record the answer in `docs/decisions.md` next to D8 either way.

### F5 · LOW · house order / tenant-existence oracle: tenant resolved before the caller
`+server.ts:24–25` resolves the tenant with the service client **before** `resolveCaller`. An
anonymous request gets 404 for an unknown org and 401 for a real one, so anyone can find out which
dōjō slugs exist. The house pattern (`resolveTenantAccess`, `dojo-auth.ts:167`) authenticates first.
**Fix:** swap the two lines and update the comment. Add a spec showing that an unauthenticated
request to an unknown tenant gets 401, not 404.

### F6 · LOW · a malformed workspace id is a 500
A non-uuid `[id]` makes `can_read_workspace` raise `22P02`. `db-refusal.ts` does not map that code,
so `callFn` turns it into a 500 "server failure" when the real cause is the caller's input. **Fix:**
check that `params.id` is a uuid in the route and return 400 (or 404). Add a spec for it.

### F7 · LOW · simplification: the same FROM/JOIN appears three times
All three branches repeat `repository_metrics ⋈ repositories_in_workspaces ⋈ repositories ⋈
metric_catalogue` with the same window filter. Put it in one `with ws as (…)` CTE and keep only the
branch predicates in each branch. The pooling `case` expression is now in two functions
(`team_member_metrics`, here). That is acceptable for SQL, but name it in both comments as "the same
rule as X" so they get changed together.

### F8 · LOW · test coverage gaps
- No case for a **team lead** (bob is a `lead` in the fixture but no case uses him). The
  `leads_team_member` arm of `can_read_repository_metric` is never reached through the rollup.
- No case where a **dōjō admin** (erin) reads workspace P. That case would show F3 for admins, who
  get every detail row.
- Add the F5/F6 cases to the route spec.

## Before commit (the handoff checklist)

1. Fix F1, F2 and F5 red-first (watch the new test fail, then fix it). Decide on F3 now, because it
   changes the row shape WP5 builds on. Ask the user about F4.
2. Converge local Supabase with `dbd reconcile` + `dbd import` (not psql). Run it twice (deploy twice).
   **Do not run it at the same time as WP6.** The plan says both reconcile the same local Supabase.
3. Run the acceptance step: with `bun run dev` in `dojo/`, call
   `GET /v1/t/organization/sensei-hq/workspaces/<dbd's workspace id>/metrics` as a member and check
   that `sensei-hq/dbd` rows come back. Call it as an outsider and expect 403. Write down the row
   count and check it against `select count(*) … where computed_on >= current_date - 30`
   (161 today).
4. `bun run check` (dōjō), `make test-fast`, then commit WP4 files only. Checkpoint afterwards.
5. WP8 ordering: the plan says WP8 changes `all_my_repositories`, which WP4 reads. If WP8 starts
   first, run this test suite again after it lands.
