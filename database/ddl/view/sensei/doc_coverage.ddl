set search_path to sensei, extensions;

-- Doc↔code traceability by filename-stem proximity, with drift detection.
--
-- ## Why this computes the pairing instead of reading stored edges
--
-- The pairing used to be built by the `build_connections` task, written into
-- `sensei.edges` as 601 `covers` rows, and then read back by THIS view — which
-- joined those rows to the very nodes they were derived from. The task's own
-- comment conceded what that made it: "covers becomes a pure function of the
-- current (docs, files) — idempotent".
--
-- A pure function of current stored state, recomputed wholesale on every index, is
-- a view that got stored. Storing it created a second writer of a fact the
-- database already held, and second writers drift — the same shape as
-- `folders.props.libs` (retired in 796a56a9), `nodes.degree` (56 of 430,988 rows
-- measurably stale) and `communities.god_node_ids`.
--
-- It also could never have been filled at insert time, which is the usual answer
-- to "don't store what the writer can compute": when `docs/api/auth.md` is
-- processed, `src/api/auth.ts` may not be indexed yet. The pairing is inherently
-- cross-file, so it is neither an insert-time fact nor barrier-worthy work.
--
-- ## The stem rule lives HERE now, not in two places
--
-- Matching by filename stem is a judgement heuristic, and this codebase's rule is
-- that judgement rules stay in Rust with one owner — a SQL *copy* of one is how
-- the scan exclusion resolver came to gate the watcher while pruning nothing. This
-- is a MOVE, not a copy: the `Path::file_stem()` call in `build_connections` is
-- gone, so exactly one implementation exists and there is nothing to drift from.
--
-- The expression below reproduces `Path::file_stem()` exactly — verified against
-- all 45,186 distinct `file_path` values in the live DB, zero disagreements,
-- including the leading-dot case (`.gitignore` is entirely its own stem) and the
-- double-extension case (`appstate.svelte.ts` → `appstate.svelte`).
--
-- `sensei.edge_kind` keeps its `covers` value: nothing produces it now, but
-- dropping an enum member requires recreating the type, and the value remains
-- valid for a future writer.
--
-- Dropped rather than replaced because the old shape carried an `edge_id` column
-- that no longer has a source — and no consumer ever read it (`get_doc_drift`
-- selects doc_name, doc_file, code_name, code_file, drifted). No other view
-- depends on this one.
drop view if exists doc_coverage;

create view doc_coverage as
with stems as (
  select n.id
       , n.folder_id
       , n.kind
       , n.name
       , n.file_path
       , n.modified_at
         -- Path::file_stem(): the file name minus its final extension; a name that
         -- begins with `.` and has no other dot is entirely the stem.
       , case
           when position('.' in substring(regexp_replace(n.file_path, '^.*/', '') from 2)) = 0
           then regexp_replace(n.file_path, '^.*/', '')
           else substring(
                  regexp_replace(n.file_path, '^.*/', '') from 1
                  for length(regexp_replace(n.file_path, '^.*/', ''))
                    - position('.' in reverse(regexp_replace(n.file_path, '^.*/', '')))
                )
         end as stem
    from nodes n
   where n.file_path is not null
     and n.kind in ('doc', 'file')
)
select d.folder_id
     , f.name            as folder
     , f.project_id
     , p.name            as project
     , p.maturity        as project_maturity
     , d.id              as doc_id
     , d.name            as doc_name
     , d.file_path       as doc_file
     , d.modified_at     as doc_modified
     , code.id           as code_id
     , code.name         as code_name
     , code.file_path    as code_file
     , code.modified_at  as code_modified
     , (code.modified_at > d.modified_at) as drifted
  from stems             d
  join stems             code
    on code.folder_id    = d.folder_id
   and code.kind         = 'file'
   and code.stem         = d.stem
   and code.file_path   <> d.file_path
  join folders           f
    on f.id              = d.folder_id
  left join projects     p
    on p.id              = f.project_id
 where d.kind            = 'doc';

comment on view doc_coverage is
'Doc-to-code traceability by filename-stem proximity, with drift detection.
drifted = true when the code was modified more recently than the covering doc.

Computes the pairing rather than reading stored `covers` edges: it is a pure
function of the current (docs, files), so storing it meant a second writer of a
fact the DB already held. The stem expression reproduces Rust Path::file_stem()
exactly — verified against all 45,186 distinct file_path values in the live DB.

Filter/group dimensions: project, project_maturity, folder, drifted.

Common queries:
  SELECT doc_file, code_file FROM doc_coverage WHERE folder = ''myrepo'' AND drifted
  SELECT folder, count(*) FILTER (WHERE drifted) as drifted FROM doc_coverage WHERE project = ''sensei'' GROUP BY folder
  SELECT project, count(*) as covered, count(*) FILTER (WHERE drifted) as drifted FROM doc_coverage GROUP BY project';
comment on column doc_coverage.drifted is 'Code modified more recently than the doc that covers it.';
comment on column doc_coverage.code_file is 'A file sharing the doc''s filename stem — docs/api/auth.md pairs with src/api/auth.ts.';
