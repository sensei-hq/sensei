set search_path to sensei, extensions;

-- How the graph's PLACED edges were placed — the sibling of graph_boundary.
--
-- `graph_boundary` answers "where does the graph stop, and why". This answers
-- "where does it hold, and on what evidence". Both are needed for the same
-- reason: a consumer handed a bare `resolved` treats the weakest claim the
-- ladder makes exactly like the strongest. An edge from `declared_here` is a
-- file pointing at its own declaration and cannot be wrong; an edge from
-- `through_a_glob` is a name the source never wrote down, and is the first
-- thing to suspect when an edge points somewhere surprising.
--
-- It is also what maps an edge back to the code that made it. A rung IS a
-- method on `Ladder` in crates/senseid/src/indexer/resolve.rs, so a wrong edge
-- tagged `through_a_glob` names `Ladder::through_a_glob` as the thing to read.
-- That is the difference between a defect report and a search.
--
-- Built for GROUP BY, same shape as graph_boundary:
--
--   select source_language, resolved_via, count(*)
--     from sensei.graph_resolution
--    where folder_id = $1
--    group by 1, 2 order by 3 desc;
--
-- reason_codes is joined LEFT for the reason graph_boundary's is: a rung with
-- no seeded prose must surface raw, never drop the edge that carries it.
-- Dropped before create: `repository_id`/`repository` sit beside `project` rather
-- than appended, and `create or replace view` can only add columns to the tail.
drop view if exists graph_resolution;

create or replace view graph_resolution as
select cg.folder_id
     , cg.folder
     , cg.project_id
     , cg.project
     , rf.repository_id
     , r.name             as repository
     , cg.edge_id
     , cg.edge_kind
     , cg.source_id
     , cg.source_name
     , cg.source_kind
     , cg.source_file
     , cg.source_line
     , cg.source_language
     , cg.target_id
     , cg.target_symbol
     , cg.target_file
     , cg.resolved_via
     , rc.precedence      as rung_precedence
     , rc.summary         as rung_summary
     , rc.detail          as rung_detail
  from call_graph cg
  left join folders rf
    on rf.id     = cg.folder_id
  left join repositories r
    on r.id      = rf.repository_id
  left join reason_codes rc
    on rc.domain = 'code_graph_rung'
   and rc.code   = cg.resolved_via
 where cg.target_id is not null;

comment on view graph_resolution is
'Every PLACED edge with the rung that placed it and the prose explaining that
rung. The sibling of sensei.graph_boundary — that one is where the graph stops,
this is where it holds and on what evidence.

Shaped for GROUP BY on folder/project/source_language/resolved_via.
reason_codes is joined LEFT so a rung with no prose surfaces raw.

resolved_via is written by the indexer into edges.props->>''rung''; the
vocabulary is sensei.reason_codes domain ''code_graph_rung'' (precedence =
climb order) and the one producer is Rung::as_label in crates/senseid.';

grant select on graph_resolution to authenticated, service_role;
