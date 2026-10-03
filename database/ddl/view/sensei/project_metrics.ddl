set search_path to sensei, extensions;

-- Compatibility view: `repository_metrics` with `project_id` restored.
--
-- The store is repository-grained (see `sensei.repository_metrics`), but a
-- project is how the app asks the question — "how is sensei doing" is a project,
-- and it may span several repositories. Rather than rewrite every dependent view
-- and read path, project_id is derived here and the whole downstream chain
-- (metric_facts, project_metric_daily → weekly/monthly/quarterly/trend,
-- metric_ratings, project_health_score …) is unchanged.
--
-- A CORRELATED SUBQUERY, not a join, and that is deliberate — now for two
-- reasons, the second of which is load-bearing enough to be a hard constraint:
--
--   1. FAN-OUT. A join to sensei.folders fans out — a repository usually has many
--      folder rows — and would multiply every metric row by its folder count,
--      silently inflating every sum downstream. The subquery yields exactly one
--      project per metric row.
--
--   2. AUTO-UPDATABILITY. `PgStore::merge_metric_explainer` runs
--      `UPDATE sensei.project_metrics SET props = …, modified_at = now()
--       WHERE id = $1` against THIS VIEW on the live explainer path. Postgres
--      auto-updates a view only when its FROM list has exactly ONE entry, so any
--      join — inner, left or lateral — silently turns that write into an error.
--      That is precisely how the 2026-09-30 migration of this view failed review
--      and was reverted (see docs/decisions.md, D-REPO-ANCHOR). A scalar subquery
--      in the SELECT list does NOT cost updatability: measured 2026-10-01,
--      `pg_relation_is_updatable` returns 28 (UPDATE+INSERT+DELETE) and the
--      explainer UPDATE above succeeds through the view.
--
-- RESOLVED THROUGH `repositories_in_projects`, THE JUNCTION ITSELF — not through
-- `sensei.folder_projects`, and not through `folders.project_id`, which this view
-- stops reading. `folders.project_id` is NOT gone: it is still a live column on
-- `sensei.folders` and other views still read it (see the view comment below and
-- docs/decisions.md, D-PROJ-JUNCTION). `folder_projects` is the right resolver for
-- a FOLDER-grained reader; this view already carries `repository_id`, so the
-- folder hop is pure cost.
--
-- Measured 2026-10-01 with `EXPLAIN (ANALYZE, BUFFERS)` on `WHERE project_id =
-- <sensei>` across all 19,581 rows, TOTAL BUFFERS (hit+read). The junction figure
-- reproduced to the block on every repetition; the two folder shapes reproduced
-- to within a handful of blocks out of three-quarters of a million, so they are
-- given to the order of magnitude they support and no further:
--
--      via the junction (this view)               59,610
--      via folders (what this replaces)          ~766,000   — ~13x more
--      via folder_projects                       ~866,000   — ~15x more
--
-- NO WALL-CLOCK IS QUOTED HERE, deliberately. `sensei.folders` is 81 MB / 10,385
-- pages against a 128 MB shared_buffers, so a scan of it rides the bulk-read ring
-- and never retains its pages: repeated runs of the identical plan varied by more
-- than 2x while their buffer counts did not move. The buffers are the evidence.
-- The cost is structural, and the plans say why: both folder shapes add a Bitmap Heap
-- Scan on `folders` returning ~205 rows per lookup (`Heap Blocks: exact=762,912`
-- via folder_projects, 722,951 via the folders subquery) to answer a question the
-- 261-row junction answers directly.
--
-- The junction's SubPlan is a SEQ SCAN of `repositories_in_projects` — 261 rows,
-- `Rows Removed by Filter: 260`, 3 pages — and that seq scan is the right plan,
-- not a missing index. 19,581 loops x 3 pages = 58,743 of the 59,610 buffers;
-- `project_repositories_repository_id_idx` does exist and the planner correctly
-- declines it, because descending an index costs more than reading a 24 kB table.
-- Both resolvers read the same authority; only this one reads it at the grain the
-- view already has.
--
-- THE PICK IS SINGLE-VALUED AND THE SOURCE IS NOT. Measured 2026-10-01: of 475
-- repositories, 253 belong to any project and 8 serve two each, carrying 681 of
-- the 19,581 metric rows. A repository is keyed on its REMOTE, so two checkouts
-- collapse to one row and membership is multi-valued BY DESIGN, not by defect.
-- This column therefore answers "one project that owns this value", NOT "the
-- project" — a convenience for the legacy downstream chain, never an authority. The
-- authoritative, multi-valued answer is `sensei.repositories_in_projects` (and
-- `sensei.folder_projects` for folder-grained readers); a reader that must not
-- miss a shared repository's values has to go there. Constraint 2 above is what
-- forces one value: the view exposes `id`, the base table's primary key, and the
-- live UPDATE targets it — fanning out would make `id` non-unique and the view
-- unwritable in the same stroke. Consequence, measured the same day: 2 projects
-- (`bridge`, `vite-multi-adapter`) are members of a metric-bearing repository yet
-- see no rows through this view. The folders-based pick it replaces shadowed 3
-- (`documentation`, `sparsh`, `vite-multi-adapter`), so this is an improvement on
-- a defect it does not pretend to cure. Removing the single-value constraint is a
-- two-step change, in this order: repoint `merge_metric_explainer` at
-- `sensei.repository_metrics` (it writes no derived column), then fan this view
-- out — at which point every dependent aggregate must be re-checked for
-- double-counting, because `project_metric_daily` groups by project_id and would
-- then legitimately pool a shared repository into both.
--
-- DEPENDENTS DO CHANGE, AND ONE CHANGE IS USER-VISIBLE. Measured 2026-10-01 by
-- loading this file inside a rolled-back transaction against the live database
-- and diffing every dependent per project. Nothing is created or lost at the
-- view's own grain — `metric_facts` stays at 19,581 rows and none resolves to
-- NULL — but ATTRIBUTION MOVES, because two repositories' picks flip:
--
--   * project_metrics rows. `bridge` 61 -> 0 and `sparsh` 0 -> 61: the `bridge`
--     repository's two memberships share a created_at, so the uuid tiebreak
--     decides, and it decides for sparsh. `client-q` 139 -> 116 and
--     `documentation` 0 -> 23: the `documentation` repository's own project was
--     claimed on 2026-10-01, later than client-q's backfilled claim.
--   * project_metric_daily 6,356 -> 6,367. bridge -31 / sparsh +31 is a pure
--     move of whole day-keys. documentation +11 are NEW group keys. client-q keeps
--     all 88 of its day-keys despite losing 23 rows, because its other
--     repositories already covered every (metric, day, grain) those rows sat on.
--   * metric_ratings 494 -> 495, and project_health_score 85 -> 86.
--
-- The user-visible one is the health score: project `bridge` LOSES ITS HEALTH
-- SCORE ENTIRELY — it had 100 on 1 rated metric and afterwards has no row at all
-- — while `documentation` and `sparsh` each gain one (also 100 on 1 rated
-- metric). A project vanishing from a health list is a screen change, not a
-- rounding change. It is the single-value pick above made visible, which is why
-- that pick is measured and written down rather than assumed.
--
-- ORDER BY created_at DESC — the MOST RECENT membership wins, and the choice was
-- measured rather than assumed. 257 of the 261 memberships were written by two
-- bulk backfill statements on 2026-09-30 (179 and 78 rows, each identical to the
-- microsecond); the 4 written individually on 2026-10-01 are per-repository
-- projects (`employee-portal`, `mycm.net`, `policy-management`, `documentation`)
-- carved out of umbrella ones (`client-h` x3, `client-q`). Preferring the later claim
-- treats the specific, deliberate act of membership as more informative than the
-- bulk backfill, and it is what shadows 2 projects instead of 6: measured the
-- same day, `created_at DESC` shadows 2 (`bridge`, `vite-multi-adapter`),
-- `created_at ASC` shadows 6, project-name order 5. The tradeoff is stated
-- plainly: unlike an ASC ordering, adding a repository to a further project MOVES
-- its historical metric rows to that project. That is the intended reading of the
-- act, but it is a move, not an addition.
--
-- THAT TIMESTAMP SPREAD IS A SNAPSHOT, NOT A CLOSED SET. The junction is no
-- longer backfill-only: since cdc10cc8 the scan path writes it directly —
-- `write_one_repo` calls `PgStore::link_project_repository` with the ids it
-- already holds — so a newly scanned repository reaches the junction and this
-- column resolves for it. Expect the created_at distribution above to keep
-- growing a tail; the ordering rule is what has to stay stable, not the counts.
--
-- `pr.project_id` is the tiebreak, and it is not decorative: 4 of the 8
-- multi-project repositories hold both memberships at the SAME created_at (same
-- backfill statement), so without it the pick would be indeterminate and could
-- differ between runs of the same query. An immutable uuid is used rather than the
-- project name because a rename must not re-attribute history.
--
-- LEFT-ish by construction: a repository with no project mapping produces
-- project_id = NULL rather than vanishing. Dropping the row would be worse — a
-- metric that exists would read as "not measured". (Measured 2026-10-01: all
-- 19,581 rows resolve to a real membership and none is NULL, so this is a guard,
-- not a live case.)
--
-- DEPLOY NOTE: adding or reordering a column here requires DROP VIEW … CASCADE
-- followed by re-applying this file and every dependent view. Postgres rejects
-- `CREATE OR REPLACE VIEW` with "cannot change name of view column" unless the
-- new column is appended last, and `dbd reconcile` only ever emits the REPLACE
-- form — so it fails mid-run, after the table changes have already landed. See
-- docs/backlog.md (dbd note, 2026-08-24). This revision changes only the
-- subquery's body, leaving every column name, type and position untouched, so it
-- applies as a plain REPLACE.
create or replace view project_metrics as
select rm.id
     , rm.metric_id
     , ( select pr.project_id
           from sensei.repositories_in_projects pr
          where pr.repository_id = rm.repository_id
          order by pr.created_at desc, pr.project_id
          limit 1
       )                                   as project_id
     , rm.repository_id
     , rm.scope
     , rm.identity
     , rm.persona_id
     , rm.commit_sha
     , rm.computed_on
     , rm.grain
     , rm.value
     , rm.props
     , rm.source
     , rm.modified_at
  from sensei.repository_metrics rm;

comment on view project_metrics is
'Read-compatibility view over sensei.repository_metrics, adding the derived
project_id. WRITES OF REAL COLUMNS GO TO THE TABLE as a matter of policy — but
note the view IS auto-updatable and one live path depends on that: measured
2026-10-01, pg_relation_is_updatable returns 28 (UPDATE+INSERT+DELETE), and
PgStore::merge_metric_explainer updates props through this view by id (verified
the same day in a rolled-back transaction: UPDATE 1). What fails loudly is naming
the DERIVED column: INSERT raises `cannot insert into column "project_id" of view
"project_metrics"` and UPDATE raises `cannot update column "project_id" of view
"project_metrics"`, both with DETAIL `View columns that are not columns of their
base relation are not updatable` — because a derived column has no inverse. So the
grain cannot be targeted wrongly, but the view must never acquire a join or it
stops being writable at all.

project_id is DERIVED FROM sensei.repositories_in_projects. This view stopped reading
folders.project_id on 2026-10-01, but THAT COLUMN IS NOT REMOVED: it is still a
live, indexed column on sensei.folders, and other views still read it. Per
docs/decisions.md D-PROJ-JUNCTION, membership now LIVES in
sensei.repositories_in_projects and folders.project_id is derived from it, written
only by inheritance from the repo anchor. The reason to stop reading it here is
that it used to be settable independently per folder, so a repository''s folders
could disagree about their project — and one pair did, the client-q
`documentation` checkout. A junction keyed on the repository makes that
disagreement unrepresentable. Because a repository may serve several projects
(measured 2026-10-01: 8 of the 253 project-owning repositories serve two), the
derived column is ONE owner, not THE owner: it takes the most recent membership,
ties broken on the project uuid. Ask sensei.repositories_in_projects, or
sensei.folder_projects, for the complete answer.

Dropped in the rename and NOT restored here: folder_id and session_id. Both held
zero rows across the table''s life and nothing downstream referenced them.';
