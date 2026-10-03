set search_path to sensei, extensions;

CREATE OR REPLACE VIEW sensei.library_usage AS
SELECT rl.library_id
     , l.name                                           AS library_name
     , f.name                                           AS folder
       -- Element 1 of a ONE-element set is that element, not a pick among
       -- candidates. Where a repository serves two projects there is no single
       -- answer, so the scalar has none and `project_ids` carries both.
     , CASE WHEN cardinality(ps.project_ids) = 1
            THEN ps.project_ids[1] END                  AS project_id
     , rl.version_used
     , (SELECT COUNT(*)
          FROM sensei.edges e
         WHERE e.folder_id = rl.folder_id
           AND e.kind = 'imports'::sensei.edge_kind
           AND e.target_id IS NULL
           AND e.target_name LIKE l.name || '::%'
       )                                                AS unresolved_import_count
     , rl.props
     , rl.modified_at
     , ps.project_ids
  FROM sensei.referenced_libraries rl
  JOIN sensei.libraries l ON l.id = rl.library_id
  JOIN sensei.folders   f ON f.id = rl.folder_id
  -- INLINE AND LATERAL, never a CTE. The aggregate has no GROUP BY, so it
  -- yields exactly one row per outer row by construction — the multi-valued
  -- source is collapsed before it can touch this view's grain.
  LEFT JOIN LATERAL (
    SELECT array_agg(fp.project_id ORDER BY fp.project, fp.project_id)
             AS project_ids
      FROM sensei.folder_projects fp
     WHERE fp.folder_id = rl.folder_id
  ) ps ON true;

comment on view library_usage is
'Per-folder library usage with import counts.
Powers the library detail panel: detected libraries, versions, and call site counts.
- library_id/library_name: which library
- folder: which repo folder uses it
- project_id/project_ids: whose it is — see below
- version_used: version from manifest (package.json, Cargo.toml)
- unresolved_import_count: unresolved IMPORT edges naming this library (approximation via name prefix).
  The kind filter is deliberate: without it the column counts any unresolved edge whose
  target_name happens to start with the library name, which is not what its name promises.
  Latent when added (only imports matched), but the shape is shared by calls and references.
- props: extensible metadata from referenced_libraries

Membership comes from `folder_projects`, never from `folders.project_id` (#211).
The old column was settable per folder, so the folders of one repository could
name different projects; `repositories_in_projects` now holds membership and
`folder_projects` resolves it through `folders.repository_id`.

THE GRAIN IS THE FOLDER AND IT DID NOT MOVE. One row per
`referenced_libraries` row, whose primary key is (folder_id, library_id). The
collapse is structural, not incidental: the lateral sub-select aggregates with
no GROUP BY, so it yields exactly one row per outer row whatever the set holds.
Measured 2026-10-02 on the live DB: 7,025 rows before and after, equal to the
7,025 rows of `referenced_libraries`, and a symmetric EXCEPT ALL over every
column the old form had except the scalar returns 0 both ways — run under
REPEATABLE READ, because `unresolved_import_count` reads a table the daemon
writes and the two sides must share one snapshot to be compared at all.
(Do not read the grain off `folder`: it is `folders.name`, not unique — 7,025
rows carry 6,285 distinct (library_id, folder-name) pairs, and that is as true
of the old form as the new, measured from both in one statement.)

Holding the grain matters because the consumer renders a list headed "Usage by
folder": a second line for the same folder, with no project column on screen to
tell them apart, would read as a duplicate and double the import count a reader
adds up by eye.

THE SCALAR IS DEFINED ONLY WHERE THE SET HAS ONE MEMBER, as in `graph_nodes` and
`doc_coverage`. `folder_projects` IS MULTI-VALUED — a repository is keyed on its
remote, so one repository can serve more than one project (measured 2026-10-02:
8 do, two projects each). Element 1 of a one-element set is that element; where
cardinality is 2 the scalar is NULL and `project_ids` carries both, because
naming one of two true answers is the drift #211 removes. Measured 2026-10-02:
1,043 of the 7,025 rows (14.8%, spread over 415 of the 1,906 libraries) read a
NULL scalar and a two-element array. No row lost its project — every one of the
1,043 rows moved their project OUT of the scalar and INTO the set, which is
the multi-project guard doing its job: those are exactly the rows whose
repository serves more than one project (measured 2026-10-02: 1,043 rows whose
repository has >1 `folder_projects` row, and 1,043 rows reading a NULL scalar
with a non-empty set — the same 1,043). NOT ONE ROW LOST ITS ATTRIBUTION: rows
with no set at all number 0. A consumer that reads only the scalar sees fewer
answers than before and must read `project_ids`; that is the price of refusing
to pick one of two true projects.

`project_ids` is APPENDED at the tail, which is the one column change
`create or replace view` allows, so no `drop view` is needed. `project_id` keeps
its name, type and position; it only became NULL-able in practice.

A LATERAL SUB-SELECT, NOT A GROUPED ONE, AND THE SPELLING IS THE WHOLE
DIFFERENCE. Written the way `doc_coverage` writes it —
`left join (select folder_id, array_agg(...) from folder_projects group by folder_id)`
— the group key is not the consumer''s key, so the planner cannot parameterise
it. EXPLAIN (ANALYZE) at `max_parallel_workers_per_gather = 0` on 2026-10-02,
on a `WHERE library_id = $1` returning 150 rows: the grouped form Seq Scans all
13,724 rows of the 81 MB `folders` heap and sorts all 17,853 `folder_projects`
rows into a GroupAggregate to do it (the merge join above stops early, so the
aggregate only emits 13,470 of the 13,724 groups — the work below it is still
done in full). The LATERAL form is correlated on `rl.folder_id` and the same statement
plans with no sequential scan of `folders` at all: Index Scan `libraries_pkey`
-> Bitmap Index Scan `referenced_libraries_library_id_idx` -> Memoize over
Index Scan `folders_pkey` -> Memoize over the lateral Aggregate, whose own
input is a 3-block Seq Scan of the 261-row `repositories_in_projects` hashed
against one `folders_pkey` probe. Those row counts and that shape are the
evidence; the two were compared on the statement MINUS
`unresolved_import_count`, which is common to both forms and dominates each
(58,562 of the 60,536 buffers the full view touches for those 150 rows, at that
fixed worker count).

`projects` is NOT eliminated from the inlined `folder_projects`, and the
tempting claim that it is would be wrong: `project` is the array''s sort key, and
a sort key is a read. It plans as an `Index Scan using projects_pkey`, inside
the same Memoize as the rest of the lateral. Ordering by name rather than by
`project_id` alone is what makes `project_ids[1]` mean the same thing here as in
`doc_coverage` and `graph_nodes`; it cannot affect the scalar, which is only
taken when there is one element.

No wall-clock is quoted. `folders` is 81 MB against a 128 MB shared_buffers, so
repeated identical runs of any shape that scans it are not comparable.

A folder under no tracked repository keeps its row here, with a NULL scalar and
a NULL array — this view answers "which folders use this library", and that is
true of a folder whose repository is untracked. Measured 2026-10-02: 0 of the
7,025 rows are in that state today, because every one of the 13,724 folders
carries a `repository_id` that resolves. `folders.repository_id` stays NULLABLE,
so that zero is the current state, not a structural guarantee.

Consequence for fixtures: a folder inserted with a NULL `repository_id` and then
wired to a project by `UPDATE folders SET project_id` still produces a row here,
but with a NULL `project_id`. A fixture asserting on the project must create a
`repositories` row, a `repositories_in_projects` row, and set `folders.repository_id`.';

comment on column library_usage.project_id is
'The project, when there is exactly one. NULL where the folder''s repository serves two projects — read `project_ids` there. Never from `folders.project_id`.';
comment on column library_usage.project_ids is
'Every project whose repository contains this folder, ordered by project name. One element for all but the 8 shared repositories; NULL if the folder is under no tracked repository.';
